# Local Forever client data probe

Operator-only Linux acquisition of `ChrClasses.db2`, `ChrRaces.db2`,
`Achievement.db2`, and explicitly opted-in validation catalogs from
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

Name-validation catalogs are a separate, explicit opt-in. They are never read by
the default command and are not part of the character-customization output
contract. Add `--ack-name-validation-tables` in any position among the optional
flags:

```bash
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-client-data esES \
  --ack-name-validation-tables
```

The option acquires all four complete files through ordinary local CascLib reads;
if any required section is unavailable, the operation aborts before that table is
saved. Each output is a newly created mode `0600` file under the private fixture
root. The probe validates each WDC5 header against the pinned `DB2Metadata.h`
and `DB2LoadInfo.h` contract, rejects sparse data, and requires the external ID
list implied by each `IndexField=-1` metadata declaration. It reports
`name_validation_schema:true`, distinct from
`character_customization_schema:true`; it never prints rows, names, keys or
decrypted values.

| file | FileDataId | layout hash | DB2Meta fields | IndexField/ParentIndexField |
| --- | ---: | ---: | ---: | ---: |
| `NamesProfanity.db2` | 1117086 | `0xF227E638` | 2/2 | -1/-1 |
| `NamesReserved.db2` | 1117085 | `0x2B2D5D97` | 1/1 | -1/-1 |
| `NamesReservedLocale.db2` | 1117087 | `0x7B9823D4` | 2/2 | -1/-1 |
| `Cfg_Categories.db2` | 1068162 | `0x8710BE94` | 6/6 | -1/-1 |

The source names are the three `Names*` tables above; there is no `Name3.db2`
table in the pinned reference. `Cfg_Categories` supplies realm charset masks.
The tables support asset acquisition only: Unicode classification, case folding,
reserved/profanity policy and locale behavior remain the C++ `ObjectMgr`/
`DB2Manager` contract and are not claimed or implemented by this probe. The
reference's `NamesReservedLocale` SQL binding is left unchanged; this mode does
not repair or reinterpret that binding.

The DB2 table hash is read from each acquired WDC header rather than invented
from source metadata. The source evidence is the pinned `DB2Metadata.h` entries
`NamesProfanityMeta`, `NamesReservedMeta`, `NamesReservedLocaleMeta` and
`Cfg_CategoriesMeta`, their `DB2LoadInfo.h` definitions, and the normal loader
path in `DB2Store.cpp`. `IndexField=-1` is C++ metadata and is deliberately not
compared with the physical WDC header `id_index` field.

Initial-character catalogs are a separate, parameterless opt-in:
`--ack-character-initialization-tables`. The default command does not read them.
The option requests complete `Map.db2`, `PowerType.db2`,
`ChrSpecialization.db2`, `ChrClassesXPowerTypes.db2`, and `Movie.db2`, plus the three unmodified text files under `gt/`:
`BaseMp.txt`, `HpPerSta.txt`, and `xp.txt`. All existing local-only, private-output,
no-overwrite and unavailable-section guards apply; no terrain is extracted.

| file | FileDataId | layout hash | DB2Meta fields | IndexField/ParentIndexField |
| --- | ---: | ---: | ---: | ---: |
| `Map.db2` | 1349477 | `0xD43AFAC3` | 26/26 | -1/-1 |
| `PowerType.db2` | 1266022 | `0x14BBEEA1` | 13/13 | 2/-1 |
| `ChrSpecialization.db2` | 1343390 | `0xDAB4CA4B` | 13/13 | 3/4 |
| `ChrClassesXPowerTypes.db2` | 1121420 | `0x70DA1F8C` | 2/1 | -1/1 |
| `Movie.db2` | 1332556 | `0xF53888FA` | 6/6 | -1/-1 |

These schema contracts come from the corresponding `DB2Metadata.h` declarations
at `02245dcd`. Table hashes are reported from the actual WDC headers, not guessed.
`character_initialization_schema:true` is acquisition evidence only, not effective
hotfix application, Player initialization or world entry. The class-power
relation's class is a parent lookup, not an invented second in-record column.
Movie acquisition supplies actual intro-movie presence; it does not play an
intro or establish scene-template/transport admission.

The text-file IDs are respectively `1391664`, `1391642`, and `1391661`, from
`map_extractor/System.cpp::ExtractGameTables`. `GameTables.h` defines 15, 1, and 5
float values per row. Acquisition checks the header column count (including the
ignored first column), preserves original bytes and reports only bounded metadata.
It additionally reports source-style Linux numeric row counts and FNV-1a
fingerprints (canonical LE f32 bits, row zero included), without emitting cells.
This does not claim Rust numeric parity or complete level coverage. The production
source loader in `GameTables.cpp::LoadGameTable` uses physical row order, not the
first-column label; a target consumer must retain that distinction.

`forever-initial-gt-oracle` replays the same source-style numeric operation on
existing private assets without CASC, SQL, network or writes. `GameTableOracle.h`
anchors `GameTables.cpp:44-109`, `Util.cpp:57-74,797-804` and
`StringConvert.h:233-258` at `02245dcd`: Linux `std::stold` followed by float
narrowing/full consumption and source invalid-value fallback. This is a
source-contrasted oracle, not a linked full C++ server or a Windows oracle.
FNV is comparison metadata, not a cryptographic guarantee.

```bash
target/forever-login/client-data-probe-build/forever-initial-gt-oracle \
  --ack-private-initial-gt-oracle "$PWD/target/forever-login/new-client-data"
FOREVER_INITIAL_GT_ORACLE_BIN="$PWD/target/forever-login/client-data-probe-build/forever-initial-gt-oracle" \
  python3 tools/wow-test-bot/client-data-probe/test_gt_oracle.py
```

Actual-file differential QA additionally requires `FOREVER_ACK_PRIVATE_DATA_TESTS=1`,
`FOREVER_CLIENT_DATA_DIRECTORY`, and the built
`FOREVER_INITIALIZATION_TABLES_BIN`. It compares all numeric bits/physical rows
in all three files, not a level-one sample. Rust intentionally uses stricter
finite-decimal asset admission; successful actual-file comparison does not erase
that boundary or prove arbitrary long-double conversion/Player formula parity.

Fresh local acquisition found three unavailable encrypted Map sections. The
strict initialization command therefore rejects Map instead of saving fake
complete data (already completed files remain private diagnostic artifacts).
An operator may additionally select `--ack-available-initial-map`, only alongside
`--ack-character-initialization-tables`. This preserves `Map.available.db2` as an
unchanged 9574-byte plaintext prefix, with all four section headers: 71 available
records and 2/5/1 unavailable records. It checks table hash `0xBD84CD62`, the
schema above and the exact observed section extents before saving; all excluded
keys must remain unavailable. Ordinary extraction remains strict.

Spell-value text inputs are a separate parameterless opt-in:
`--ack-spell-value-game-tables`. It does not imply the initialization or spell
DB2 groups. It preserves complete `gt/SpellScaling.txt` (FileDataId 1391660,
24 numeric columns), `gt/CombatRatingsMultByILvl.txt` (1391670, four) and
`gt/StaminaMultByILvl.txt` (1980632, four). Source contracts are
`System.cpp::ExtractGameTables` and `GameTables.h` at `02245dcd`. Existing
private-output/no-overwrite/header/numeric guards apply; the shared metadata
field `initial_game_table_header` reports the same header guard for this group.
No level samples, zero-filled encrypted sections or numeric cells are emitted.

The existing read-only oracle has a second closed acknowledgement mode:

```bash
target/forever-login/client-data-probe-build/forever-initial-gt-oracle \
  --ack-private-spell-value-gt-oracle "$PWD/target/forever-login/new-spell-value-data"
FOREVER_ACK_PRIVATE_DATA_TESTS=1 \
FOREVER_INITIAL_GT_ORACLE_BIN="$PWD/target/forever-login/client-data-probe-build/forever-initial-gt-oracle" \
FOREVER_CLIENT_DATA_DIRECTORY="$PWD/target/forever-login/new-spell-value-data" \
FOREVER_SPELL_VALUE_TABLES_BIN="<built forever_spell_value_tables example>" \
  python3 tools/wow-test-bot/client-data-probe/test_gt_oracle.py SpellValueGameTableOracle
```

Select that class for a spell-value-only asset directory; the initialization
class expects different files. Without actual-file acknowledgement/consumer,
the two actual differential cases are skipped, not passed. All three tables
must parse before either consumer emits its result. Physical row zero is
included, labels do not select rows, and row counts do not establish a playable
level cap. These inputs do not establish CalcValue or SpellInfo readiness.

This corresponds to `DB2FileLoader.cpp::LoadTableData`'s explicit
`DB2EncryptedSectionHandling::Skip` path at `02245dcd`, not zero-filling or
rewriting a full table. Unknown maps cannot be declared known absent or admitted
for character creation. The bounded artifact is locale-specific to the observed
70170/esES file; this does not establish a generic partial-DB2 capability.

Skill and starting-loadout tables are a separate, parameterless opt-in:
`--ack-character-birth-tables`. It requests five complete files, not spell or
item templates. The default command and existing opt-ins remain unchanged.
`AcquisitionSchemas.h` owns the metadata contracts; `ProbeOptions.h` owns
acknowledgement parsing. `main.cpp` remains the single local-acquisition/output/
validation owner. No runtime/SQL writer is added.

```bash
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-birth-data esES \
  --ack-character-birth-tables --ack-available-achievements
```

| file | FileDataId | layout hash | DB2Meta fields | IndexField/ParentIndexField |
| --- | ---: | ---: | ---: | ---: |
| `SkillLine.db2` | 1240935 | `0x6763217C` | 15/15 | 5/-1 |
| `SkillRaceClassInfo.db2` | 1240406 | `0x24277B48` | 7/7 | -1/0 |
| `SkillLineAbility.db2` | 1266278 | `0x224F7EA0` | 18/18 | 2/3 |
| `CharacterLoadout.db2` | 1344281 | `0x713CE8BB` | 5/5 | -1/-1 |
| `CharacterLoadoutItem.db2` | 1302846 | `0x0C7A1862` | 2/2 | -1/0 |

The source is `DB2Metadata.h` at `02245dcd`, with consumers in
`ObjectMgr.cpp:3968-4125` and `Player.cpp:5768-6038,25544-25735`. All existing
private/no-overwrite/no-download/unavailable-section guards apply. The probe
reports `character_birth_schema:true`, actual header hashes and bounded counts,
never rows or mask values. In-record parents need not have a physical parent
lookup, and metadata index fields are not the native header index fields.

These are acquisition prerequisites only: effective official/custom hotfixes
and removals, SQL `skill_tiers`, full `SpellInfo`/`ItemTemplate`, recursive
profession synchronization/spell learning and inventory placement/persistence
remain distinct requirements. No empty spellbook, fabricated item, or completed
Player is implied by successful extraction. An unavailable required section is
rejected; the ordinary birth-table mode has no automatic fallback or zero fill.

The observed `SkillLineAbility` file has 7833 plaintext rows and five one-row
sections without local keys. A separate `--ack-available-birth-abilities` opt-in,
only with `--ack-character-birth-tables`, requests the bounded source-`Skip`
artifact `SkillLineAbility.available.db2`. It preserves the original prefix
through byte 346918 and all six section headers, checking exact target
hashes/flags/ID/parent lookup, section offsets and the plaintext parent-lookup
extent. All excluded keys must remain unavailable. `BirthAbilityPrefix.h`
owns this narrow admission gate and synthetic drift/extent tests; the main
acquisition owner retains CASC, key-availability checks and private file writes.
There is no generic encrypted-table relaxation. The five excluded rows remain
unknown, not known absent or synthesized. Successful header acquisition is
not Rust decoding, effective catalogs or birth spell/skill acceptance.

The read-only `wow-data` example `forever_birth_tables` consumes those numeric
types with the same explicit prefix choice and emits only known/unknown counts.
Its Rust build/tests are currently pending; the following private-copy QA is
written, **not executed**:

```bash
FOREVER_ACK_PRIVATE_DATA_TESTS=1 \
  FOREVER_CLIENT_DATA_DIRECTORY="$PWD/target/forever-login/new-birth-data" \
  FOREVER_BIRTH_TABLES_BIN="$PWD/target/debug/examples/forever_birth_tables" \
  python3 tools/wow-test-bot/client-data-probe/test_rust_birth.py
```

That QA checks actual-file counts, all five hashes/layouts, required opt-ins,
strict no-fallback behavior, sparse/extent/key-marker rejection and uint16
parent bounds. Counts/negative guards are not full value differential or
skill/spell/inventory parity. Temporary copies alone are modified.

Item quantities/instantiation prerequisites have a separate parameterless opt-in:
`--ack-character-item-tables`. It requests complete `Item`, `ItemSparse`,
`ItemEffect` and `ItemXItemEffect`, using `DB2Metadata.h` at `02245dcd`:

| file | FileDataId | layout hash | DB2Meta fields | IndexField/ParentIndexField |
| --- | ---: | ---: | ---: | ---: |
| `Item.db2` | 841626 | `0x9A2A4834` | 16/16 | -1/-1 |
| `ItemSparse.db2` | 1572924 | `0x6FCC3191` | 68/68 | -1/-1 |
| `ItemEffect.db2` | 969941 | `0x4CA77678` | 9/9 | -1/-1 |
| `ItemXItemEffect.db2` | 3177687 | `0x96F083AD` | 2/1 | -1/1 |

Defaults/existing opt-ins are unchanged; unknown/repeated options remain errors.
All complete-file, local-only, private-root/no-overwrite guards still apply,
including the existing 4 MiB file bound. Sparse files or unavailable sections
reject the operation; there is **no** partial-item or zero-fill mode. Native
table hashes are reported from the acquired headers, not copied from 3.4.3.
Successful extraction alone does not establish Rust decoding, effective hotfixes,
full ItemTemplate (specs/durability/stats/bonuses/addons), use/equip or save.

`--ack-item-template-tables` is a separate complete-file opt-in for source
`ObjectMgr::LoadItemTemplates` specialization/relic inputs. It does not change
the default or the four-table item group. Contracts from `DB2Metadata.h` at
`02245dcd` are ItemSpec (FDID 1135120, layout `83F3D113`, 6/6 fields, external
ID/in-record parent 2), ItemSpecOverride (1134576, `B292998C`, 2/1 fields,
external ID/extra parent 1), and GemProperties (1343604, `86487AD2`, 2/2 fields,
external ID/no parent). Fresh complete acquisition supplies 0/9/0 records.
The empty files are real complete 228/212-byte tables with primitive field
metadata, not failed reads interpreted as absence. Zero-section reads follow
`DB2FileLoader.cpp:1788-1802,1860-1915`; contradictory counts/blobs/extent and
missing metadata are rejected. Optional metadata-only inspection can target
this group too, without saving its records. New complete artifacts remain
0600/0700, ignored, no-overwrite, local-only; no installation/account is modified.

Working Rust raw/effective spec readers and derived numeric template rules
(durability/specs/effects/addons) are implemented but **uncompiled/unexecuted**.
Valid hotfix metadata requires full record serialization, including ItemSparse
localized strings; the existing rejection fence is not removed. These inputs
are not item instantiation/equip/use/save or native world-entry evidence.

`--ack-item-table-metadata-only`, only with the item-table acknowledgement,
reports the four actual headers/section counts/key availability without reading
or saving their record bodies. It checks source header layout/field metadata,
not sparse rows, section ID lists or full numeric decoding. `file_saved:false`
and `normal_reader_supported:false` explicitly distinguish observations from
complete acquisition. Only bounded header buffers are allocated (204 bytes plus
at most 1024 section headers); the complete-file 4 MiB bound is unchanged.
The normal base files and requested Achievement handling remain unchanged.
Fresh local diagnostics find unavailable sections in all four item stores and
a 6,999,130-byte **sparse** ItemSparse (flags 5). The normal decoder must not
silently accept it; source-backed sparse/explicit encrypted-Skip handling is
still a prerequisite to the real item producer. No item file was saved by
these metadata diagnostics.

`--ack-available-item-tables`, only alongside the item-table acknowledgement,
selects four exact build-70170/esES original prefixes, following the target
loader's explicit encrypted-section Skip. It cannot combine with metadata-only
inspection. Complete mode remains strict, and neither option is automatic.
`ItemPrefixes.h` owns the captured hash/layout/flags/field/section/key-marker/
record/copy/extent gates, with synthetic drift and short/overlong tests.

| output | prefix bytes | available direct records / copies | unknown direct records |
| --- | ---: | --- | ---: |
| `Item.available.db2` | 301411 | 9033 / 22788 | 59 |
| `ItemSparse.available.db2` | 6971318 | 19167 / 57 | 69 |
| `ItemEffect.available.db2` | 125074 | 7580 / 5015 | 40 |
| `ItemXItemEffect.available.db2` | 189486 | 12588 / 0 | 40 |

Only this exact ItemSparse prefix exceeds the normal complete-file 4 MiB
bound. Files/header/section bytes are preserved unchanged; no zero fill, row
synthesis, decryption bypass or sparse regular-reader fallback is introduced.
All excluded sections must still lack keys before saving. Source anchors are
`DB2FileLoader.cpp:990-1050,1858-1968` at `02245dcd`: sparse catalog loading
reads catalog IDs, copies, six-byte entries, **then an additional ID table**.
Both ID lists must be included in the prefix extent. Full ItemTemplate and
inventory remain pending; numeric readers/effective SQL composition now exist
in working code but have not passed Rust or actual-file acceptance.

The new `wow-data::forever_birth::item_sparse` baseline consumer is implemented
but **not compiled/tested yet**. It independently scans target variable strings
and all numeric cells/arrays, follows sparse catalog/copy order and retains the
69-row unknown diagnostic. The counts-only `forever_sparse_items` example needs
`--ack-private-sparse-item-prefix <private-directory>`; it never prints records,
admits a Player or saves inventory. Its actual-file result and complete numeric
differential are pending; successful extraction is not those results.

`wow-data::forever_birth::item_records::ItemRecords::load_available` now
consumes all four frozen prefixes before returning. Its counts-only example
`forever_item_tables` requires `--ack-private-item-prefixes <private-directory>`.
The working target startup also loads eight official/custom numeric SQL queries
and applies final removals into one immutable catalog, with explicit
`--ack-available-item-tables`. These Rust paths are **uncompiled/unexecuted**;
they neither instantiate/equip inventory nor admit/save a Player. Acquisition
test results do not certify them or full ItemTemplate/wire serialization.

The target read-only Rust consumer is the `forever_initialization_tables`
example in `wow-data`. It requires `--ack-local-client-data <private-directory>`
and, for the bounded Map artifact, an additional `--ack-available-initial-map`.
It reports only baseline/GT counts, never rows, and does not apply SQL hotfixes
or admit a Player. Its implementation is currently unvalidated; do not treat
the acquisition-tool tests as having executed that Rust consumer.

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

SpellInfo prerequisites are an independent, parameterless opt-in:
`--ack-spell-info-tables`. The exact **42** source metadata contracts live in
`SpellInfoSchemas.h`, anchored to `SpellMgr::LoadSpellInfoStore`, `SpellInfo`
constructors, their learn/form/category dependencies and the six custom-attribute inputs at `02245dcd`.
This is not full SpellInfo, SQL spell data, casting or a learned Player.
No spell store is extracted by default and no 3.4.3 spell layout is substituted.
Ordinary complete acquisition remains strict: an unavailable section aborts
before that spell table is saved, and the complete-file 4 MiB bound remains.

`--ack-spell-table-metadata-only` additionally requires the spell group. It
reports bounded real header/section metadata, actual table hashes, locale masks,
parent lookup counts and key availability, with `file_saved:false` for each
spell file. It does not inspect record bodies or certify full decoding.
Normal classes/races and selected Achievement handling remain unchanged.

The current real 70170/esES observation finds 24 tables with unavailable sections and
18 with no unavailable section. The original 36-table acquisition had 20/16. A separate opt-in
`--ack-available-spell-info-tables`, only alongside the spell group, acquires
the **twenty-four exact** readable prefixes declared in `SpellPrefixes.h` and the
other eighteen files through ordinary complete reads. It cannot combine with
spell metadata-only and rejects a storage locale other than observed esES.
The prefix gate checks the captured schema/hash/flags/count/extent, source
normal string/ID/copy/relationship boundaries and all excluded keys. Numeric
headers have locale mask `0xFFFFFFFF`; SpellName/BattlePetSpecies/SpellItemEnchantment have mask
64. The storage-selection locale is not substituted for that native field.
No generic encrypted/sparse recovery, zero fill, record synthesis or ordinary
reader relaxation is introduced. Unknown direct rows/copies remain unknown.

```bash
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-spell-data esES \
  --ack-spell-info-tables --ack-available-spell-info-tables \
  --ack-available-achievements
```

Actual available/unknown direct counts include SpellName 17565/569,
SpellEffect 42409/1261, SpellMisc 31720/906 and SpellPower 3429/37.
SpellName also retains 14173 readable copies. Eighteen complete files include
six genuine empty tables with primitive metadata, not failed reads treated
as absence. The isolated CMake/header-prefix self-test and 14 CLI guards pass;
acknowledged local acquisition succeeds. This establishes **acquisition only**,
not Rust decoding, effective overlays/removals, skill learning, character save
or native initial-world acceptance. Assets stay private and uncommitted.

The working Rust implementation now adds all 49 typed raw spell record families
and closed checked schemas in `wow-data::forever_spells`. These inputs are not
effective SQL data, assembled SpellInfo or learned Player state. Its explicit
available mode preserves unknown counts; complete mode never falls back to
prefixes. Ten new Rust tests and the read-only `forever_spell_tables` example
are written but **not built/run**. Ten opt-in actual-file tests are in
`test_rust_spells.py`, using an explicitly built example, acknowledged input
under this checkout's private root and disposable copies only:

```bash
FOREVER_ACK_PRIVATE_DATA_TESTS=1 \
FOREVER_CLIENT_DATA_DIRECTORY="$PWD/target/forever-login/<acquired-spell-directory>" \
FOREVER_SPELL_TABLES_BIN="$PWD/target/debug/examples/forever_spell_tables" \
  python3 tools/wow-test-bot/client-data-probe/test_rust_spells.py
```

This command remains unexecuted and is scheduled at completed-delivery
acceptance, not as a separate per-helper Cargo campaign. QA emits counts only;
it covers all table schema/locale gates, truncation/missing files, strict mode,
unknown-section promotion, empty primitive metadata and bounded full reads.
Copy counts are bounds, not an invented guarantee that every copy adds an ID.


The six-dependency extension on 2026-10-03 adds Talent and LiquidType complete
reads plus four exact prefixes for SpellItemEnchantment, SpellVisual,
SpellVisualMissile and SpellVisualEffectName. Metadata-only observation at
09:46–09:47 UTC and fresh acknowledged acquisition at 09:58–09:59 UTC both
exit zero. The rebuilt tool's header/prefix self-test and 14 CLI guards pass.
The new output remains ignored/private (directory 0700, files 0600), separate
from the retained original 36-table extraction. Acquisition proves only known
bytes/counts, not Rust decoding or complete custom attributes. Raw/effective
stores, SQL conversion, eight locale families and full serializers are written
but uncompiled/unexecuted; the source-native missile-set index borrows the same
final canonical records. This 42-table checkpoint expanded the ten actual-file
QA cases; they remain unrun until completed-delivery acceptance.

The following extension at 10:16–10:27 UTC adds six complete numeric stores:
ExpectedStat (1937326), ExpectedStatMod (1969773), ContentTuning (1962930),
ContentTuningXExpected (2976765), RandPropPoints (1310245) and MythicPlusSeason
(2400282). Exact contracts are in `SpellInfoSchemas.h`, from pinned
DB2Metadata/LoadInfo/Structure. All six observed 70170/esES files are complete;
no new prefix guard is added. The explicit spell-info group now requests
48 inputs: 24 complete files and 24 admitted known prefixes. Fresh metadata/
acquisition, header self-test and 14 CLI guards pass. Ten actual-file Rust QA
cases now require the new 48-table extraction; no Rust consumer was run.

Working Rust adds all 87 new numeric SQL columns (642 total), six serializers,
final canonical composition and source-contrasted expected-stat/item-value
calculation. SQL queries read existing hotfix overlays; acquisition does not
import DB2 files into SQL or modify the database. Metadata inspection is
read-only. Twenty-three new Rust cases are authored but unexecuted. These
inputs/calculations are not complete CalcValue, custom attributes, Player/save
or native-world acceptance. See the owning runbook for exact source anchors.

Outputs are private client assets, not repository fixtures. Never stage them.

At 11:32–11:34 UTC on 2026-10-03, the spell-info acquisition group expands to
49 stores with complete `UnitCondition.db2` (1120959, layout `0x215FAF83`,
table hash `0x0E540EFD`, four array-bearing fields, external ID/no parent).
The existing native tool was rebuilt with one job. The first invocation was
rejected for missing `--ack-spell-info-tables` before output creation; the
corrected acknowledged local acquisition exits zero. Observed UnitCondition
metadata is 346 direct rows, zero unavailable rows and seven copies. There are
24 exact prefixes and 25 complete spell files, plus classes/races/Achievement
(52 private files total). Directory/file modes were checked as 0700/0600 for
the acquisition root and UnitCondition. No client login or DB import occurred.
`test_rust_spells.py` now requires this 49-table input and remains unexecuted.
The authored pipeline adds 26 existing SQL columns (668 total) and all 52
delivery bytes; acquisition does not prove Rust decoding or native delivery.
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
