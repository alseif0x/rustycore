# World Entry Implementation: BNet Login to Player Movement

A comprehensive reference documenting every step of the Rust WorldServer implementation
that takes a WoW 3.4.3.54261 client from Battle.net authentication through character
selection and into the game world with working movement.

**Target client:** WoW 3.4.3 build 54261 (2023 Wrath of the Lich King Classic)
**Reference implementation:** RustyCore reference C# (TrinityCore-based)
**Status:** Fully verified -- client connects, authenticates, lists/creates/deletes
characters, enters the world, sees the player, and can move.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Login Packet Sequence](#2-login-packet-sequence)
3. [UpdateObject Deep Dive](#3-updateobject-deep-dive)
4. [Encryption](#4-encryption)
5. [Compression](#5-compression)
6. [ConnectTo Flow](#6-connectto-flow)
7. [Critical Bugs Found and Fixed](#7-critical-bugs-found-and-fixed)
8. [C# vs Rust Packet Size Comparison](#8-c-vs-rust-packet-size-comparison)
9. [Known Gaps](#9-known-gaps)
10. [Testing Strategy](#10-testing-strategy)

---

## 1. Architecture Overview

### The Complete Flow

```
WoW Client
    |
    |  TCP (TLS 1.2 over port 1119)
    v
[bnet-server] ── REST (axum) + RPC (protobuf framing)
    |              - Account authentication (SRP6)
    |              - Realm list (JSON with prefixed type names)
    |              - Realm join ticket (game account name, NOT email)
    v
[world-server] ── TCP (port 8085, realm socket)
    |              - Connection string handshake
    |              - AuthChallenge / AuthSession (HMAC-SHA256)
    |              - EnterEncryptedMode (Ed25519ctx signed)
    |              - Session init packets (AuthResponse, etc.)
    |              - Character list/create/delete
    |              - PlayerLogin -> ConnectTo
    v
[world-server] ── TCP (port 8086, instance socket)
    |              - Connection string handshake (same as realm)
    |              - AuthContinuedSession (HMAC-SHA256 with ConnectToKey)
    |              - EnterEncryptedMode
    |              - ResumeComms
    |              - Full login sequence (~34 packets)
    |              - UpdateObject (player creation)
    |              - World interaction (movement, etc.)
    v
Player visible in world, movement working
```

### Crate Architecture

```
world-server (binary)
    |-- wow-world       (WorldSession, handlers/character.rs)
    |-- wow-network     (WorldSocket, accept loop, instance listener, SessionManager)
    |-- wow-packet      (WorldPacket, headers, compression, all packet types)
    |-- wow-handler     (inventory-based dispatch table)
    |-- wow-database    (sqlx pools, CharacterDatabase, LoginDatabase)
    |-- wow-crypto      (WorldCrypt AES-GCM, HMAC-SHA256, Ed25519ctx, RSA signing)
    |-- wow-constants   (opcodes: ClientOpcodes, ServerOpcodes)
    |-- wow-core        (ObjectGuid, PackedGuid, Position)
    |-- wow-config      (config file loading)
    |-- wow-logging     (tracing setup)
```

```
bnet-server (binary)
    |-- wow-proto       (prost-build protobuf, 11 .proto files)
    |-- wow-database    (LoginDatabase)
    |-- wow-crypto      (GruntSRP6)
    |-- wow-config, wow-logging, wow-core
```

### Key Design Decisions

- **Callback pattern for session creation:** `wow-network` cannot depend on `wow-world`
  (circular). The accept loop takes a callback `on_session_ready(AccountInfo, pkt_rx, send_tx,
  resources)` that `world-server` implements to create a `WorldSession`.

- **Split socket I/O:** After authentication, `WorldSocket` splits into `SocketReader` +
  `SocketWriter` with independent `WorldCrypt` instances. Each has its own AES-GCM counter
  (critical -- see Section 4).

- **Channel-based packet flow:** `SocketReader` decrypts packets and sends them via
  `flume::Sender<WorldPacket>` to the session. `WorldSession` sends responses via
  `flume::Sender<Vec<u8>>` to `SocketWriter`, which encrypts and writes to TCP.

- **Inventory-based handler dispatch:** Handlers register via `inventory::submit!` macros
  with opcode, session status, and processing type. The dispatch table is built once at
  startup from all registered entries.

---

## 2. Login Packet Sequence

### Phase 0: BNet Authentication (bnet-server)

| # | Direction | Description |
|---|-----------|-------------|
| 1 | C -> S | REST POST `/bnetserver/login/` (email + SRP6 proof) |
| 2 | S -> C | REST response (SRP6 server proof, session ticket) |
| 3 | C -> S | RPC `RealmListTicketRequest` (with JSONRealmListTicketClientInformation) |
| 4 | S -> C | RPC response (ticket granted) |
| 5 | C -> S | RPC `RealmListRequest` |
| 6 | S -> C | RPC response (JSONRealmListUpdates with realm info) |
| 7 | C -> S | RPC `RealmJoinRequest` (realm ID) |
| 8 | S -> C | RPC response (JSONRealmListServerIPAddresses, session key stored in DB) |

**Critical:** The `Param_RealmJoinTicket` is the **game account name** (e.g., "2#1"), NOT
the email. The session key is stored as a **64-byte raw BLOB**, not a hex string.

### Phase 1: Realm Socket Handshake (world-server, port 8085)

| # | Direction | Packet | Notes |
|---|-----------|--------|-------|
| 1 | S -> C | `"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"` | Raw string, 53 bytes |
| 2 | C -> S | `"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n"` | Raw string, 53 bytes |
| 3 | S -> C | `SMSG_AUTH_CHALLENGE` | 16-byte server challenge + 32-byte DoS challenge |
| 4 | C -> S | `CMSG_AUTH_SESSION` | Realm join ticket + HMAC digest + local challenge |
| 5 | S -> C | `SMSG_ENTER_ENCRYPTED_MODE` | Ed25519ctx signature (64 bytes) |
| 6 | C -> S | `CMSG_ENTER_ENCRYPTED_MODE_ACK` | Empty acknowledgment |

After step 6, all packets are AES-128-GCM encrypted. Counters start at 2 (see Section 4).

### Phase 2: Session Init (first encrypted packets, realm socket)

These 8 packets are sent by `WorldSession::send_session_init_packets()` immediately after
encryption is enabled:

| # | Opcode | Packet | Purpose |
|---|--------|--------|---------|
| 1 | `SMSG_AUTH_RESPONSE` | AuthResponse (OK) | Confirms auth success, sends realm info, available classes |
| 2 | `SMSG_SET_TIME_ZONE_INFORMATION` | SetTimeZoneInformation | Timezone strings (UTC) |
| 3 | `SMSG_FEATURE_SYSTEM_STATUS_GLUE_SCREEN` | FeatureSystemStatusGlueScreen | Char select features (NOT in-game version) |
| 4 | `SMSG_CLIENT_CACHE_VERSION` | ClientCacheVersion | Cache ID = 24081 |
| 5 | `SMSG_AVAILABLE_HOTFIXES` | AvailableHotfixes | Empty (no hotfixes) |
| 6 | `SMSG_ACCOUNT_DATA_TIMES` | AccountDataTimes | Global (empty GUID), 15 timestamps |
| 7 | `SMSG_TUTORIAL_FLAGS` | TutorialFlags | 8x u32 (all 0xFFFFFFFF = all shown) |
| 8 | `SMSG_CONNECTION_STATUS` | ConnectionStatus | State=1, SuppressNotification=true |

At this point the client shows the character select screen and sends `CMSG_ENUM_CHARACTERS`.

### Phase 3: Character Select Interactions

| Direction | Packet | Notes |
|-----------|--------|-------|
| C -> S | `CMSG_ENUM_CHARACTERS` | List characters |
| S -> C | `SMSG_ENUM_CHARACTERS_RESULT` | Character list with equipment cache |
| C -> S | `CMSG_CREATE_CHARACTER` | Name + race/class + customizations |
| S -> C | `SMSG_CREATE_CHAR` | Response code + GUID |
| C -> S | `CMSG_CHAR_DELETE` | Character GUID |
| S -> C | `SMSG_DELETE_CHAR` | Response code |
| C -> S | `CMSG_PLAYER_LOGIN` | Character GUID (triggers ConnectTo flow) |

The client also sends several "fire-and-forget" opcodes during character select:
- `CMSG_SERVER_TIME_OFFSET_REQUEST` -- answered with `SMSG_SERVER_TIME_OFFSET`
- `CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS` -- answered with undelete status
- `CMSG_BATTLE_PAY_GET_PRODUCT_LIST` -- silently ignored (stub)
- `CMSG_BATTLE_PAY_GET_PURCHASE_LIST` -- silently ignored (stub)
- `CMSG_UPDATE_VAS_PURCHASE_STATES` -- silently ignored (stub)
- `CMSG_SOCIAL_CONTRACT_REQUEST` -- silently ignored (stub)
- `CMSG_DB_QUERY_BULK` -- answered with `SMSG_DB_REPLY` (Status::Invalid per record)
- `CMSG_HOTFIX_REQUEST` -- answered with empty `SMSG_HOTFIX_CONNECT`

### Phase 4: ConnectTo Flow (see Section 6 for details)

| # | Direction | Packet | Socket |
|---|-----------|--------|--------|
| 1 | S -> C | `SMSG_CONNECT_TO` | Realm |
| 2 | C disconnects from realm socket | | |
| 3 | C -> S | Connection strings | Instance |
| 4 | S -> C | `SMSG_AUTH_CHALLENGE` | Instance |
| 5 | C -> S | `CMSG_AUTH_CONTINUED_SESSION` | Instance |
| 6 | S -> C | `SMSG_ENTER_ENCRYPTED_MODE` | Instance |
| 7 | C -> S | `CMSG_ENTER_ENCRYPTED_MODE_ACK` | Instance |

### Phase 5: Login Sequence (instance socket, ~34 packets)

Sent by `WorldSession::send_login_sequence()`. The exact order matches C#
`HandlePlayerLogin` -> `SendInitialPacketsBeforeAddToMap` -> `AddToMap` ->
`SendInitialPacketsAfterAddToMap`:

| # | Opcode | Packet | Phase |
|---|--------|--------|-------|
| 0 | -- | `SMSG_RESUME_COMMS` | (ConnectTo only) |
| 1 | `SMSG_DUNGEON_DIFFICULTY_SET` | DungeonDifficultySet | HandlePlayerLogin |
| 2 | `SMSG_LOGIN_VERIFY_WORLD` | LoginVerifyWorld | HandlePlayerLogin |
| 3 | `SMSG_ACCOUNT_DATA_TIMES` | AccountDataTimes (player) | HandlePlayerLogin |
| 4 | `SMSG_FEATURE_SYSTEM_STATUS` | FeatureSystemStatus (in-game) | HandlePlayerLogin |
| 5 | `SMSG_BATTLE_PET_JOURNAL_LOCK_ACQUIRED` | BattlePetJournalLockAcquired | HandlePlayerLogin |
| 6 | `SMSG_TIME_SYNC_REQUEST` | TimeSyncRequest (seq=0) | BeforeAddToMap |
| 7 | `SMSG_CONTACT_LIST` | ContactList (empty) | BeforeAddToMap |
| 8 | `SMSG_BIND_POINT_UPDATE` | BindPointUpdate | BeforeAddToMap |
| 9 | `SMSG_SET_PROFICIENCY` | SetProficiency (weapons) | BeforeAddToMap |
| 10 | `SMSG_SET_PROFICIENCY` | SetProficiency (armor) | BeforeAddToMap |
| 11 | `SMSG_UPDATE_TALENT_DATA` | UpdateTalentData (empty) | BeforeAddToMap |
| 12 | `SMSG_SEND_KNOWN_SPELLS` | SendKnownSpells (empty) | BeforeAddToMap |
| 13 | `SMSG_SEND_UNLEARN_SPELLS` | SendUnlearnSpells | BeforeAddToMap |
| 14 | `SMSG_SEND_SPELL_HISTORY` | SendSpellHistory (empty) | BeforeAddToMap |
| 15 | `SMSG_SEND_SPELL_CHARGES` | SendSpellCharges (empty) | BeforeAddToMap |
| 16 | `SMSG_ACTIVE_GLYPHS` | ActiveGlyphs (empty, full) | BeforeAddToMap |
| 17 | `SMSG_UPDATE_ACTION_BUTTONS` | UpdateActionButtons (180 zeros) | BeforeAddToMap |
| 18 | `SMSG_INITIALIZE_FACTIONS` | InitializeFactions (1000 factions) | BeforeAddToMap |
| 19 | `SMSG_SETUP_CURRENCY` | SetupCurrency (empty) | BeforeAddToMap |
| 20 | `SMSG_LOAD_EQUIPMENT_SET` | LoadEquipmentSet (empty) | BeforeAddToMap |
| 21 | `SMSG_ALL_ACCOUNT_CRITERIA` | AllAccountCriteria (empty) | BeforeAddToMap |
| 22 | `SMSG_ALL_ACHIEVEMENT_DATA` | AllAchievementData (empty) | BeforeAddToMap |
| 23 | `SMSG_LOGIN_SET_TIME_SPEED` | LoginSetTimeSpeed | BeforeAddToMap |
| 24 | `SMSG_WORLD_SERVER_INFO` | WorldServerInfo | BeforeAddToMap |
| 25a | `SMSG_SET_FLAT_SPELL_MODIFIER` | SetSpellModifier (flat, empty) | BeforeAddToMap |
| 25b | `SMSG_SET_PCT_SPELL_MODIFIER` | SetSpellModifier (pct, empty) | BeforeAddToMap |
| 26 | `SMSG_ACCOUNT_MOUNT_UPDATE` | AccountMountUpdate (empty) | BeforeAddToMap |
| 27 | `SMSG_ACCOUNT_TOY_UPDATE` | AccountToyUpdate (empty) | BeforeAddToMap |
| 28 | `SMSG_INITIAL_SETUP` | InitialSetup (expansion=2) | BeforeAddToMap |
| 29 | `SMSG_MOVE_SET_ACTIVE_MOVER` | MoveSetActiveMover | BeforeAddToMap |
| 30 | `SMSG_UPDATE_OBJECT` | UpdateObject (CREATE_OBJECT2) | AddToMap |
| 31 | `SMSG_INIT_WORLD_STATES` | InitWorldStates | AfterAddToMap |
| 32 | `SMSG_LOAD_CUF_PROFILES` | LoadCufProfiles (empty) | AfterAddToMap |
| 33 | `SMSG_AURA_UPDATE` | AuraUpdate (empty) | AfterAddToMap |

After packet 33, the session state transitions to `LoggedIn` and periodic `TimeSyncRequest`
packets begin (5s interval for first 2, then 10s).

### Post-Login Periodic Packets

| Direction | Packet | Interval |
|-----------|--------|----------|
| S -> C | `SMSG_TIME_SYNC_REQUEST` | 5s (first 2), then 10s |
| C -> S | `CMSG_TIME_SYNC_RESPONSE` | Response to above |
| C -> S | `CMSG_PING` | Periodic (~30s) |
| S -> C | `SMSG_PONG` | Inline response (handled in socket reader, bypasses session) |

---

## 3. UpdateObject Deep Dive

The `SMSG_UPDATE_OBJECT` packet is the most complex packet in the protocol. It creates
the player's visible representation in the game world.

### Top-Level Envelope

```
[u32]  NumObjUpdates       -- number of object updates in this packet
[u16]  MapID               -- map the updates apply to
[...]  Data buffer          -- see below
```

### Data Buffer Layout

```
[bit]  HasDestroyOrOutOfRange
  if true:
    [u16]  destroyCount     -- objects to destroy (client plays death animation)
    [i32]  totalCount       -- destroy + out-of-range combined
    [PackedGuid...]         -- destroy GUIDs, then out-of-range GUIDs
[i32]  dataBlockSize        -- byte size of the concatenated update blocks
[bytes] update blocks       -- concatenated CreateObject/Values/etc blocks
```

### CreateObject Block

Each block for `UpdateType::CreateObject2` (value 2):

```
[u8]   UpdateType           -- 2 for CreateObject2
[PackedGuid] ObjectGUID     -- 128-bit packed GUID
[u8]   TypeId               -- 7 (ActivePlayer) for self, 6 (Player) for others
[18 bits] CreateObjectBits  -- see below
[...]  MovementUpdate       -- if bit 3 set
[i32]  PauseTimes count     -- always 0 (written regardless of flags)
[...]  ActivePlayer block   -- if bit 16 set (is_self only) -- THE 721-BYTE FIX
[...]  ValuesCreate         -- field data (no change masks for CREATE)
```

### 18-Bit CreateObjectBits

Written in this exact order (bit 0 first):

| Bit | Name | Set for Player? |
|-----|------|-----------------|
| 0 | NoBirthAnim | false |
| 1 | EnablePortals | false |
| 2 | PlayHoverAnim | false |
| 3 | MovementUpdate | **true** (has position/speeds) |
| 4 | MovementTransport | false |
| 5 | Stationary | false |
| 6 | CombatVictim | false |
| 7 | ServerTime | false |
| 8 | Vehicle | false |
| 9 | AnimKit | false |
| 10 | Rotation | false |
| 11 | AreaTrigger | false |
| 12 | GameObject | false |
| 13 | SmoothPhasing | false |
| 14 | ThisIsYou | **true** (self only) |
| 15 | SceneObject | false |
| 16 | ActivePlayer | **true** (self only) |
| 17 | Conversation | false |

### MovementUpdate (bit 3)

```
[PackedGuid]  MoverGUID
[u32]         MovementFlags        -- 0 for standing still
[u32]         MovementFlags2       -- 0
[u32]         ExtraMovementFlags2  -- 0
[u32]         MoveTime             -- 0
[f32]         Position.X
[f32]         Position.Y
[f32]         Position.Z
[f32]         Position.Orientation
[f32]         Pitch                -- 0.0
[f32]         StepUpStartElevation -- 0.0 (MUST be f32, NOT u32!)
[u32]         RemoveForcesIDs.Count -- 0
[u32]         MoveIndex            -- 0
[7 bits]      Conditional flags:
              HasStandingOnGameObjectGUID = false
              HasTransport = false
              HasFall = false
              HasSpline = false
              HeightChangeFailed = false
              RemoteTimeValid = false
              HasInertia = false
[9 x f32]    Movement speeds:
              Walk=2.5, Run=7.0, RunBack=4.5, Swim=4.72222,
              SwimBack=2.5, Fly=7.0, FlyBack=4.5,
              TurnRate=PI, PitchRate=PI
[i32]         MovementForces.Count -- 0
[f32]         ModMagnitude         -- 1.0
[17 x f32]   AdvancedFlying parameters:
              AirFriction=2.0, MaxVel=65.0, LiftCoefficient=1.0,
              DoubleJumpVelMod=3.0, GlideStartMinHeight=10.0,
              AddImpulseMaxSpeed=100.0, MinBankingRate=90.0,
              MaxBankingRate=140.0, MinPitchingRateDown=180.0,
              MaxPitchingRateDown=360.0, MinPitchingRateUp=90.0,
              MaxPitchingRateUp=270.0, MinTurnVelThreshold=30.0,
              MaxTurnVelThreshold=80.0, SurfaceFriction=2.75,
              OverMaxDeceleration=7.0, LaunchSpeedCoefficient=0.4
[bit]         HasSplineData -- false
[FlushBits]
```

### ActivePlayer Movement Block (bit 16) -- THE CRITICAL 721 BYTES

This block is written AFTER PauseTimes and BEFORE ValuesCreate, but ONLY when
the `ActivePlayer` bit (bit 16) is set. Missing this block was the cause of the
most severe crash (ACCESS_VIOLATION) -- see Section 7.

```
[bit]  HasSceneInstanceIDs  -- false
[bit]  HasRuneState         -- false
[bit]  HasActionButtons     -- true
[FlushBits]                  -- 1 byte (3 bits + 5 padding)
[180 x u32] ActionButtons   -- 720 bytes (all zeros for fresh character)
                             -- Total: 1 + 720 = 721 bytes
```

Without this block, the client reads the ValuesCreate data at the wrong offset,
interpreting field data as position/size values, leading to an immediate crash.

### ValuesCreate Block

Format: `[u32 size][u8 flags][section data...]`

The `flags` byte controls conditional fields:
- `0x01` (Owner) -- set for self-view, adds extra fields in UnitData
- `0x04` (PartyMember) -- set for party members, adds QuestLog in PlayerData

**For CREATE operations, there are NO change masks.** All fields are written
sequentially. This differs from VALUES updates which use bitmask-based change tracking.

#### Section Order

1. **ObjectData** (3 fields, 12 bytes):
   - EntryId (i32) = 0
   - DynamicFlags (u32) = 0
   - Scale (f32) = 1.0

2. **UnitData** (~170 fields, variable size depending on Owner flag):
   - Health/MaxHealth (i64 each -- NOT i32!)
   - DisplayId (i32)
   - NpcFlags[2] (u32 each)
   - StateSpellVisualID, StateAnimID, StateAnimKitID (i32)
   - StateWorldEffectIDs.Count (i32 = 0)
   - 9-10 PackedGuids (Charm, Summon, [Critter if Owner], CharmedBy, etc.)
   - BattlePetDBID (u64)
   - ChannelData (i32 + i32)
   - Race, ClassId, PlayerClassId, Sex, DisplayPower (u8 each)
   - Power[10], MaxPower[10], ModPowerRegen[10]
   - Level, EffectiveLevel, scaling fields (9x i32)
   - FactionTemplate (i32)
   - VirtualItems[3]
   - Flags, Flags2, Flags3, AuraState
   - AttackRoundBaseTime[2]
   - BoundingRadius, CombatReach, DisplayScale (f32)
   - Stats[5], Resistances[7] (Owner only)
   - ModCastingSpeed through ModTimeRate (6x f32, all 1.0)
   - AttackPower block (Owner only -- 13 fields)
   - Various misc fields, GuildGUID
   - CurrentAreaID (u32 = zone_id)
   - ComboTarget (Owner only)

3. **PlayerData** (~50 fields, ~300 bytes):
   - DuelArbiter, WowAccount, LootTargetGUID (PackedGuids)
   - PlayerFlags, PlayerFlagsEx
   - Guild info, Customizations.Size
   - PartyType[2], bank/PvP info
   - VisibleItems[19]
   - AvgItemLevel[6]
   - DungeonScoreSummary (f32 + f32 + i32)

4. **ActivePlayerData** (self only, ~11000 bytes):
   - InvSlots[141] (PackedGuids)
   - FarsightObject, SummonedBattlePetGUID
   - Coinage (i64), XP, NextLevelXP (i32)
   - SkillInfo: 256 entries x 7 u16s = 3584 bytes
   - ExploredZones[240] (u64 each) = 1920 bytes
   - QuestCompleted[875] (u64 each) = 7000 bytes
   - CombatRatings[32] (i32)
   - BuybackPrice[12] + BuybackTimestamp[12]
   - Honor kills (8x u16)
   - PvpInfo[7] (each: i8 + 16 fields + bit)
   - Various collection sizes (all 0)
   - FrozenPerksVendorItem
   - GlyphSlots[6] + Glyphs[6]
   - ResearchSites, trailing bits

---

## 4. Encryption

### Algorithm: AES-128-GCM with 12-byte Tags

WoW 3.4.3 uses AES-128-GCM but with **12-byte tags** instead of the standard 16-byte tags.

```rust
// CORRECT -- 12-byte nonce, 12-byte tag
type WowAesGcm = AesGcm<Aes128, U12, U12>;

// WRONG -- standard 16-byte tag, WILL FAIL
type WowAesGcm = Aes128Gcm;  // This is AesGcm<Aes128, U12, U16>
```

**Never truncate or pad GCM tags.** The last 4 bytes of a 16-byte tag are NOT zeros -- they
are part of the cryptographic computation. Use the correct generic parameter.

### Packet Wire Format (encrypted)

```
[4 bytes]  Size (i32 LE)           -- size of encrypted data
[12 bytes] Tag                      -- AES-GCM authentication tag
[N bytes]  Encrypted data           -- AES-GCM ciphertext
```

Header total: 16 bytes (4 size + 12 tag).

### Nonce Format

```
[8 bytes]  Counter (u64 LE)        -- increments per packet
[4 bytes]  Suffix (u32 LE)         -- direction identifier

Server -> Client: suffix = "SRVR" = 0x52565253
Client -> Server: suffix = "CLNT" = 0x544E4C43
```

### Counter Offset: Starting at 2, Not 0

This was one of the most subtle bugs. In C#, `PacketCrypt.Encrypt()` and `Decrypt()`
**always** increment the counter, even when `IsInitialized == false` (before encryption
is enabled):

```csharp
public bool Encrypt(ref byte[] data, ref byte[] tag) {
    if (IsInitialized)
        _serverEncrypt.Encrypt(nonce, data, data, tag);
    ++_serverCounter;  // ALWAYS increments, even without encrypting!
    return true;
}
```

During the handshake, 2 unencrypted packets are sent/received per direction:

- **Server -> Client:** AuthChallenge (counter 0->1) + EnterEncryptedMode (counter 1->2)
- **Client -> Server:** AuthSession (counter 0->1) + EnterEncryptedModeAck (counter 1->2)

Therefore, the **first encrypted packet** uses counter=2 in both directions.

```rust
// Writer (server->client): start at packets sent unencrypted
WorldCrypt::new_with_server_counter(&key, self.unencrypted_packets_sent)

// Reader (client->server): start at packets received unencrypted
WorldCrypt::new_with_client_counter(&key, self.unencrypted_packets_received)
```

### Key Derivation

After `AuthSession` validation:

1. **Digest key hash:** `SHA256(keyData || platformAuthSeed)`
   - `keyData` = 64-byte raw session key from DB
   - `platformAuthSeed` = build-specific seed (from `build_info` table, per platform)

2. **Auth check:** `HMAC-SHA256(digestKeyHash, localChallenge || serverChallenge || AuthCheckSeed)`
   - Compare first 24 bytes with client's digest

3. **Session key (40 bytes):**
   ```
   keyHash = SHA256(keyData)
   seed = HMAC-SHA256(keyHash, serverChallenge || localChallenge || SessionKeySeed)
   sessionKey = SessionKeyGenerator256(seed).generate(40)
   ```

4. **Encryption key (16 bytes):**
   ```
   full = HMAC-SHA256(sessionKey, localChallenge || serverChallenge || EncryptionKeySeed)
   encryptKey = full[0..16]
   ```

### Ed25519ctx Signing (EnterEncryptedMode)

The `EnterEncryptedMode` packet includes a 64-byte signature computed using Ed25519ctx
(RFC 8032 with `phflag=0` and context bytes):

```
toSign = HMAC-SHA256(encryptKey, [enabled_byte] || EnableEncryptionSeed)
signature = Ed25519ctx_sign(privateKey, toSign, EnableEncryptionContext)
```

This is NOT standard Ed25519 -- the context parameter changes the signature completely.

### Protocol Constants (16 bytes each)

```
AUTH_CHECK_SEED:         C5 C6 98 95 76 3F 1D CD B6 A1 37 28 B3 12 FF 8A
SESSION_KEY_SEED:        58 CB CF 40 FE 2E CE A6 5A 90 B8 01 68 6C 28 0B
CONTINUED_SESSION_SEED:  16 AD 0C D4 46 F9 4F B2 EF 7D EA 2A 17 66 4D 2F
ENCRYPTION_KEY_SEED:     E9 75 3C 50 90 93 61 DA 3B 07 EE FA FF 9D 41 B8
ENABLE_ENCRYPTION_SEED:  90 9C D0 50 5A 2C 14 DD 5C 2C C0 64 14 F3 FE C9
ENABLE_ENCRYPTION_CONTEXT: A7 1F B6 9B C9 7C DD 96 E9 BB B8 21 39 8D 5A D4
```

---

## 5. Compression

### When Compression Applies

Packets larger than `COMPRESSION_THRESHOLD` (0x400 = 1024 bytes) are compressed before
encryption. The compressed packet is sent with the `SMSG_COMPRESSED_PACKET` opcode wrapping
the original packet data.

### Compressed Packet Format

```
[u16]  CompressedPacket opcode    -- 0x0446 (part of encrypted data)
[i32]  UncompressedSize           -- original opcode(2) + payload size
[u32]  UncompressedAdler32        -- custom Adler32 of original data
[u32]  CompressedAdler32          -- custom Adler32 of compressed data
[...]  CompressedData             -- raw deflate with Z_SYNC_FLUSH
```

### Persistent Deflate Stream

C# TrinityCore uses a single `z_stream` per socket, initialized once with
`deflateInit2(stream, 1, 8, -15, 8, 0)` and reused for all packets. The WoW client has
a matching persistent inflate stream. Each compressed packet must use the same deflate
state:

```rust
pub struct PacketCompressor {
    inner: Compress,  // flate2 raw deflate, level 1 (fast)
}
```

Each packet is flushed with `FlushCompress::Sync` (Z_SYNC_FLUSH), which appends the
sync marker `00 00 FF FF` to the compressed data. The client's persistent inflate
stream processes this marker to know where each packet ends.

**Creating a fresh compressor per packet will NOT work** -- the client's decompressor
carries state across packets, so the server's compressor must too.

### Custom Adler32

WoW uses a non-standard Adler32 initial value:

```rust
const ADLER32_INIT: u32 = 0x9827_D8F1;
```

The Adler32 is computed incrementally: first over the opcode bytes, then continued over
the payload bytes using the intermediate state from the opcode computation.

### Compression in Practice

During a typical login, these packets exceed the threshold and get compressed:
- `SMSG_UPDATE_ACTION_BUTTONS` (~1443 bytes: 2 opcode + 1 bit + flush + 180*8)
- `SMSG_INITIALIZE_FACTIONS` (~6127 bytes: 1000 factions)
- `SMSG_UPDATE_OBJECT` (~15624 bytes for self-view ActivePlayer)

---

## 6. ConnectTo Flow

The ConnectTo flow separates the "realm" connection (character select) from the "instance"
connection (gameplay). This is the architecture WoW uses to support seamless world transfers.

### Overview

```
1. Client sends CMSG_PLAYER_LOGIN on realm socket
2. Server generates ConnectToKey (packed: accountId + connectionType + random)
3. Server registers pending entry in SessionManager
4. Server signs ConnectTo with RSA and sends SMSG_CONNECT_TO
5. Client disconnects from realm socket
6. Client connects to instance port
7. Client sends AuthContinuedSession (with ConnectToKey + HMAC digest)
8. Instance listener validates against SessionManager
9. Instance listener delivers InstanceLink via oneshot channel
10. Session swaps send_tx to instance socket channel
11. Session sends ResumeComms + full login sequence on instance socket
```

### ConnectToKey Format (bit-packed i64)

```
Bits [0..20]   = accountId (u32, truncated to 21 bits)
Bits [20..21]  = connectionType (1 = Instance)
Bits [21..52]  = random key (31 bits)
```

### RSA Signing

The `SMSG_CONNECT_TO` packet includes a 256-byte RSA-SHA256 signature. The signed data is:

```
[u8]    address type (1 = IPv4)
[4B]    IP address bytes
[u16]   port (LE)
[...plus padding to match C# format]
```

The RSA private key is derived from hardcoded C# `RsaStore` constants (PKCS#1 DER format).

### SessionManager

Thread-safe `DashMap<u32, PendingEntry>` indexed by account ID:

```rust
pub struct SessionManager {
    pending: DashMap<u32, PendingEntry>,
}

struct PendingEntry {
    connect_to_key: i64,
    session_key: Vec<u8>,
    instance_link_tx: oneshot::Sender<InstanceLink>,
}

pub struct InstanceLink {
    pub send_tx: flume::Sender<Vec<u8>>,     // session writes here
    pub pkt_tx: flume::Sender<WorldPacket>,   // reader forwards here
}
```

Flow:
1. `register(account_id, key, session_key)` -- returns `oneshot::Receiver<InstanceLink>`
2. Instance listener calls `validate_and_take(account_id, key)` -- consumes entry
3. Instance listener sends `InstanceLink` via the oneshot sender
4. Session's `poll_instance_link()` receives the link and swaps channels

### Fallback: Direct Login

If no `SessionManager` is configured (e.g., single-server setup), the session falls back
to direct login on the realm socket:

```rust
fn fallback_direct_login(&mut self) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let link = InstanceLink {
        send_tx: self.send_tx().clone(),
        pkt_tx: flume::bounded(1).0,
    };
    let _ = tx.send(link);
    self.set_instance_link_rx(Some(rx));
}
```

This creates a dummy oneshot that fires immediately with the existing send channel,
so `poll_instance_link()` triggers `handle_continue_player_login()` on the next update.

**Important:** In direct login mode, `ResumeComms` must NOT be sent -- the client did
not go through ConnectTo and does not expect it. Sending it causes a disconnect.

### ConnectToFailed Retry

If the client sends `CMSG_CONNECT_TO_FAILED`, the server retries with the next serial:
- `WorldAttempt1` -> `WorldAttempt2` -> `WorldAttempt3` -> `WorldAttempt4` -> `WorldAttempt5`
- After all 5 attempts, falls back to direct login

### Instance Listener Authentication

The instance listener (`start_instance_listener()`) performs the same cryptographic
handshake as the realm socket but uses `AuthContinuedSession` instead of `AuthSession`:

1. Connection string exchange (identical)
2. `AuthChallenge` sent with new server challenge
3. Client sends `AuthContinuedSession` with:
   - `ConnectToKey.Raw` (i64)
   - Local challenge (16 bytes)
   - HMAC-SHA256 digest (24 bytes)
4. Server validates:
   - Extract account_id from ConnectToKey
   - Look up in SessionManager, verify key matches
   - Compute HMAC-SHA256(SHA256(sessionKey), key || localChallenge || serverChallenge || ContinuedSessionSeed)
   - Compare first 24 bytes with client's digest
5. Derive encryption key (same formula as realm, but with new local/server challenges)
6. Send `EnterEncryptedMode`, wait for ack
7. Deliver `InstanceLink` via oneshot

---

## 7. Critical Bugs Found and Fixed

### Bug 1: ACCESS_VIOLATION from Missing ActivePlayer Movement Block (Phase 7)

**Severity:** Critical -- client crashes immediately on world entry

**Symptom:** Client reaches loading screen, then crashes with `ACCESS_VIOLATION` during
`UpdateObject` processing. The crash occurred inside the client's field reader, trying
to interpret data at completely wrong offsets.

**Root cause:** The `ActivePlayer` bit (bit 16) in CreateObjectBits was set, but the
corresponding block in `BuildMovementUpdate` was not written. In C#, when
`flags.ActivePlayer` is true, a 721-byte block is written between PauseTimes and
ValuesCreate:

- 3 bits: HasSceneInstanceIDs (false) + HasRuneState (false) + HasActionButtons (true)
- FlushBits (1 byte)
- 180 action buttons (4 bytes each = 720 bytes)

Without this block, the ValuesCreate data started 721 bytes earlier than the client expected.
The client would read the first few fields of ObjectData as garbage values, including reading
into memory that did not belong to the packet buffer.

**Fix:** Added `write_active_player_movement_block()` in update.rs, called after PauseTimes
when `is_self == true`.

### Bug 2: AES-GCM 16-byte Tags vs 12-byte Tags (Phase 6)

**Severity:** Critical -- bidirectional encryption failure

**Symptom:** `AES-GCM decryption failed` in both directions. Client silently disconnects.

**Root cause:** Rust's `Aes128Gcm` type alias produces 16-byte tags, but WoW 3.4.3 uses
12-byte tags. The C# code explicitly configures `new AesGcm(key, tagSizeInBytes: 12)`.
Attempted workaround of padding 12->16 bytes with zeros failed because the last 4 bytes
are part of the cryptographic computation, not zeros.

**Fix:** Use the generic type with explicit 12-byte tag size:
```rust
type WowAesGcm = AesGcm<Aes128, U12, U12>;
```

### Bug 3: Counter Offset Starting at 2 (Phase 6)

**Severity:** Critical -- first encrypted packet in each direction fails to decrypt

**Symptom:** Server->Client: client cannot decrypt. Client->Server: `decrypt failed,
counter=0` on the first encrypted packet.

**Root cause:** C# increments the counter in Encrypt()/Decrypt() OUTSIDE the
`if (IsInitialized)` check. Two unencrypted packets per direction means the counter
reaches 2 before encryption starts.

**Fix:** Track `unencrypted_packets_sent` and `unencrypted_packets_received` in
WorldSocket. Pass these as offsets when creating `WorldCrypt` instances for the
split reader/writer.

### Bug 4: MySQL VARBINARY from Binary Collation (Phase 6)

**Severity:** High -- session thread panics, client disconnects

**Symptom:** `ColumnDecode: Rust type String is not compatible with SQL type VARBINARY`

**Root cause:** The `name` column in `characters` has `COLLATE utf8mb4_bin` (binary
collation for case-sensitive searches). MySQL reports this as `VARBINARY` to sqlx,
which rejects decoding it as `String`.

**Fix:** Added `read_string()` to `SqlResult` that tries `String` first, then falls back
to `Vec<u8>` with `String::from_utf8_lossy()`:
```rust
pub fn read_string(&self, column: usize) -> String {
    if let Some(s) = self.try_read::<String>(column) { return s; }
    if let Some(bytes) = self.try_read::<Vec<u8>>(column) {
        return String::from_utf8_lossy(&bytes).into_owned();
    }
    String::new()
}
```

**Rule:** ALWAYS use `read_string()` for MySQL text columns in WoW databases.

### Bug 5: SQL Schema Mismatches (Phase 6)

**Guild ID:** The `guildid` column does not exist in `characters`. Guild membership is
in the `guild_member` table and must be `LEFT JOIN`ed.

**Column name:** The schema defines `at_login`, not `atLoginFlags`.

**Deleted characters:** Must include `WHERE deleteInfos_Name IS NULL` to filter
soft-deleted characters.

**Fix:** Corrected all queries to match the actual DDL in `sql/base/characters_database.sql`.

### Bug 6: MoveSetActiveMover Missing (Phase 7)

**Severity:** High -- client crashes when trying to move

**Symptom:** After world entry, attempting any movement (WASD, mouse click) causes
`ACCESS_VIOLATION`. The player appears in the world but cannot move.

**Root cause:** The `SMSG_MOVE_SET_ACTIVE_MOVER` packet was not sent. This packet tells the
client which unit it controls for movement input. Without it, the client's `m_mover` pointer
is null, and any movement input triggers a null pointer dereference.

**Fix:** Added `MoveSetActiveMover { mover_guid: guid }` to the login sequence, matching
C#'s `SetMovedUnit(this)` call at Player.cs line 5610.

### Bug 7: StepUpStartElevation Type (Phase 5)

**Severity:** Medium -- subtle field offset corruption

The `StepUpStartElevation` field in MovementUpdate was initially written as u32 (integer 0)
instead of f32 (float 0.0). While both produce the same bytes for value 0, the type matters
for documentation accuracy and would cause issues if ever set to non-zero values.

### Bug 8: DbQueryBulk "Streaming Error" (Phase 7)

**Severity:** Medium -- client gets stuck at 80% loading

**Symptom:** When entering the world, the client sends `CMSG_DB_QUERY_BULK` requesting
DB2 records. If no response is sent, the client tries to download from CDN, which fails
and produces "Streaming Error" -- the loading bar gets stuck at ~80%.

**Fix:** Respond with `SMSG_DB_REPLY` with `Status::Invalid` for every requested record.
This tells the client to use its local DB2 data instead of trying to stream from CDN.

---

## 8. C# vs Rust Packet Size Comparison

The initial Rust `UpdateObject` packet was 19193 bytes vs C#'s 15624 bytes -- a difference
of 3569 bytes. Breakdown of the key differences and how they were resolved:

### Size Contributors

| Section | Self-view Size | Notes |
|---------|---------------|-------|
| Envelope (NumUpdates + MapID + flags + sizes) | ~15 bytes | |
| CreateObject header (type + guid + typeid) | ~6 bytes | |
| 18-bit CreateObjectBits | 3 bytes (2.25 + flush) | |
| MovementUpdate | ~168 bytes | MoverGUID + flags + position + 9 speeds + 17 AdvFlying + bits |
| PauseTimes | 4 bytes | i32 count = 0 |
| ActivePlayer movement block | **721 bytes** | 3 bits + flush + 180 action buttons |
| ObjectData | 12 bytes | |
| UnitData (with Owner) | ~700 bytes | Variable due to Owner-conditional fields |
| PlayerData | ~300 bytes | |
| ActivePlayerData | ~11000 bytes | SkillInfo(3584) + ExploredZones(1920) + QuestCompleted(7000) + rest |
| **Total (self-view)** | **~15600 bytes** | |

### Why 19193 vs 15624?

The early Rust implementation had several extra fields and incorrect sizes:

1. **Extra bytes in UnitData:** Some fields were written unconditionally that should be
   Owner-only (PowerRegen, Stats, Resistances, etc.)
2. **Incorrect array sizes:** Some arrays had wrong counts (e.g., PowerCostMultiplier
   was 10 instead of 7)
3. **Missing conditional logic:** The `is_owner` flag controls many field groups in
   UnitData. Non-self views omit significant chunks.
4. **Duplicate fields:** Some fields were written twice during iterative development.

After careful C# line-by-line comparison, the Rust packet size converged to match C# exactly.

---

## 9. Known Gaps

### Character Creation Missing Data

The current character creation inserts only the `characters` table row and customizations.
A fully functional character also needs:

| Table | Data | Impact |
|-------|------|--------|
| `character_skills` | Starting skills per race/class | No skills on character |
| `character_spell` | Starting spells per race/class | Empty spellbook |
| `character_action` | Default action bar buttons | Empty action bar |
| `character_inventory` | Starting equipment | Naked character |
| `character_homebind` | Hearthstone bind location | No hearthstone |
| `playercreateinfo` | Should be read from world DB | Hardcoded instead |
| `playercreateinfo_item` | Starting items per race/class | No items |

### Missing Packets vs C# Full Implementation

The C# server sends many more packets during world entry that the Rust server currently
does not:

| Packet | Purpose | Impact of Missing |
|--------|---------|-------------------|
| `SMSG_PHASE_SHIFT_CHANGE` | Phase system | No phasing support |
| `SMSG_SPELL_GO` / `SMSG_SPELL_START` | Spell cast visuals | No spell effects |
| `SMSG_POWER_UPDATE` | Power bar updates | Power bar may be wrong |
| `SMSG_SET_MOVEMENT_ANIM` | Movement animations | May have animation glitches |
| `SMSG_OBJECT_UPDATE` (VALUES type) | Field updates | No dynamic updates |
| `SMSG_WEATHER` | Zone weather | No weather |
| `SMSG_ZONE_UNDER_ATTACK` | PvP zone status | No PvP zone info |
| `SMSG_UPDATE_WORLD_STATE` | World state variables | No world states |

### Movement Handling

The server currently handles movement by:
- Sending `MoveSetActiveMover` (so client can input movement)
- Responding to `Ping` with `Pong` (keeping connection alive)
- Sending periodic `TimeSyncRequest` (keeping time in sync)

But it does NOT yet:
- Process `CMSG_MOVE_*` packets (movement validation)
- Broadcast movement to other players
- Handle teleportation
- Process `CMSG_SET_ACTIVE_MOVER`

### Other Missing Systems

- Chat system (no CMSG_CHAT_MESSAGE_* handlers)
- Spell casting
- Combat
- NPC interaction
- Group/guild operations
- Mail system
- Auction house
- Loot
- Quest system
- Achievement tracking (packets sent empty but no tracking)

---

## 10. Testing Strategy

### Approach: C# as Reference Implementation

The primary testing strategy was to run the C# server as the "known good" reference and
compare the Rust implementation against it:

1. **BNet server first:** Build and verify the Rust BNet server against the C# WorldServer.
   This confirmed the login flow, session key generation, and realm join process work
   correctly before touching the WorldServer.

2. **Packet-level comparison:** Enable hex dump logging in both C# and Rust servers, then
   compare packet-by-packet:
   - Same opcodes sent in same order
   - Same payload sizes
   - Byte-level comparison for critical packets (UpdateObject, AuthResponse, etc.)

3. **Incremental testing:** Each phase was tested independently:
   - Phase 2: BNet connects to C# WorldServer
   - Phase 3-4: Rust WorldSocket accepts connections from client
   - Phase 6: Client reaches character select screen
   - Phase 7: Client enters world

### Packet Logging

Both `SocketReader` and `SocketWriter` log every packet with hex dumps:

```
Writer[127.0.0.1:54321]: PKT#2 opcode=0x2583 len=3602
HEX: 83 25 01 00 00 00 ...

Reader[127.0.0.1:54321]: received opcode 0x3583, size=46
HEX: 83 35 ...
```

This allowed direct comparison with C# logs to identify discrepancies.

### Unit Tests

Each crate has unit tests covering:
- **wow-packet:** Serialization/deserialization roundtrips, envelope format, size assertions
- **wow-crypto:** Key derivation, HMAC, Ed25519ctx signatures, WorldCrypt encrypt/decrypt
- **wow-network:** Session manager register/validate/remove, connection string constants
- **wow-world:** Session state transitions, legit character management, packet sending
- **wow-packet/compression:** Adler32, compress/decompress roundtrip, persistent stream,
  sync marker verification

Total: 323 tests across the workspace.

### Key Test Assertions

```rust
// UpdateObject self-view must be much larger than other-view
assert!(self_bytes.len() > other_bytes.len() + 1000);

// ActivePlayer movement block adds exactly 721 bytes
let diff = self_bytes.len() - other_bytes.len();
assert!(diff > 721);

// Compressed data must end with Z_SYNC_FLUSH marker
assert_eq!(last4, &[0x00, 0x00, 0xFF, 0xFF]);

// Persistent compressor produces valid output for sequential packets
// (tested with 3 packets through single decompressor)
```

### Build and Test Commands

```bash
export PATH="/home/cdmonio/.cargo/bin:/usr/bin:/usr/local/bin:/bin:$PATH"
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test --workspace
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo check -p world-server
```

### Debugging Checklist

When something fails in the connection flow:

1. **Client disconnects silently after handshake:**
   - Verify GCM tag size (12 vs 16 bytes)
   - Verify counter offsets (count pre-encryption packets)
   - Verify Ed25519ctx signature (not standard Ed25519)

2. **Decrypt failures in reader:**
   - Check client_counter offset
   - Verify nonce suffix (SRVR vs CLNT)
   - Verify key derivation matches C# output

3. **Character list errors:**
   - Check SQL column names against DDL
   - Use LEFT JOINs for related tables
   - Include `deleteInfos_Name IS NULL`
   - Use `read_string()` for text columns

4. **Session thread panics:**
   - Replace `unwrap()` with `try_read()` / `read_string()`
   - Handlers must never panic -- always return error responses

5. **Client crashes on world entry:**
   - Compare packet sizes byte-by-byte with C#
   - Check that ActivePlayer movement block (721 bytes) is present
   - Verify `MoveSetActiveMover` is sent before movement input
   - Check `DbQueryBulk` responses (prevents "Streaming Error")

6. **Client stuck at 80% loading:**
   - Missing `DbQueryBulk` responses -- respond with Status::Invalid
   - Missing `TimeSyncRequest` -- must be sent before UpdateObject
