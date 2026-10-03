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
field counts, table/layout hashes and acquisition mode. Header checks follow
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
