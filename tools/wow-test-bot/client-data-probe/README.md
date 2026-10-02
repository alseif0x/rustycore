# Local Forever client data probe

Operator-only Linux acquisition of `ChrClasses.db2` and `ChrRaces.db2` from
installed `wow_classic_beta` build 70170. It uses the reference's pinned CascLib,
requires a new output directory inside the ignored `target/forever-login`,
does not import missing encryption keys or substitute zero-filled data, and
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
