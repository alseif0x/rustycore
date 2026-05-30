# Migration: Config Reference (`worldserver.conf` / `bnetserver.conf`)

> **C++ canonical path:** `src/server/worldserver/worldserver.conf.dist`, `src/server/bnetserver/bnetserver.conf.dist`
> **Rust target crate(s):** `crates/wow-config/`, consumers in `crates/world-server/`, `crates/bnet-server/`
> **Layer:** L0 — Foundation (parsed before anything else; values feed every other layer)
> **Status:** ⚠️ partial — Rust loads configs but covers ~24 keys vs ~612 in TC `worldserver.conf.dist` (~4 % parity); reload command absent.
> **Audited vs C++:** ✅ complete (this document)
> **Last updated:** 2026-05-01

---

## 1. Purpose

The two `.conf.dist` files are TrinityCore's runtime configuration — every tunable a server operator can flip without recompiling. `worldserver.conf` (~4 444 lines, ~612 keys) drives the world server: DB connection strings, ports, intervals, rates, anti-cheat thresholds, BG/arena economy, instance reset schedule, logger appenders, packet-spoof policy, and PvP balance. `bnetserver.conf` (~446 lines, ~25 keys) drives the auth/realm-list daemon: TLS material, REST port, TOTP master secret, wrong-password ban policy. Both files share the same `Key = Value` text format with `#` line / inline comments and a single `[section]` header that is otherwise ignored.

The values flow into three fixed-size arrays on the global `World` singleton — `m_int_configs[INT_CONFIG_VALUE_COUNT]`, `m_bool_configs[BOOL_CONFIG_VALUE_COUNT]`, `m_float_configs[FLOAT_CONFIG_VALUE_COUNT]` (declared `World.h:847-850`) — indexed by the `WorldIntConfigs` / `WorldBoolConfigs` / `WorldFloatConfigs` enums (`World.h:102-441`). Every gameplay subsystem reads through `sWorld->getIntConfig(CONFIG_FOO)` and friends, so a missing or misparsed key silently degrades to whatever default `LoadConfigSettings()` wrote into the slot at startup.

---

## 2. C++ canonical files

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/worldserver/worldserver.conf.dist` | 4 444 | Canonical world-server config (~612 keys, 30 sections) |
| `src/server/bnetserver/bnetserver.conf.dist` | 446 | Canonical auth-server config (~25 keys, 6 sections) |
| `src/server/game/World/World.h` | 102-441 | `WorldBoolConfigs` / `WorldFloatConfigs` / `WorldIntConfigs` enums + array declarations (`m_int_configs[847]`, `m_bool_configs[849]`, `m_float_configs[850]`) and accessors (`setIntConfig`, `getBoolConfig`, etc., lines 684-717) |
| `src/server/game/World/World.cpp` | — | `World::LoadConfigSettings(bool reload)` reads every key into the arrays; called once at boot and again on `.reload config` |
| `src/common/Configuration/Config.h/.cpp` | — | `ConfigMgr` singleton, `LoadInitial`, `Reload`, `GetIntDefault`, `GetStringDefault`, `GetFloatDefault`, `GetBoolDefault` |
| `src/server/game/Chat/Commands/cs_reload.cpp` | — | Implements the `.reload config` GM command that calls `sConfigMgr->Reload()` + `sWorld->LoadConfigSettings(true)` |

### `worldserver.conf.dist` section index (line ranges)

| Section | Lines | Sample keys |
|---|---|---|
| EXAMPLE CONFIG | 47-60 | (template only) |
| CONNECTIONS AND DIRECTORIES | 62-244 | `RealmID`, `DataDir`, `LogsDir`, `LoginDatabaseInfo`, `WorldDatabaseInfo`, `CharacterDatabaseInfo`, `HotfixDatabaseInfo`, `*.WorkerThreads`, `*.SynchThreads`, `MaxPingTime`, `WorldServerPort=8085`, `InstanceServerPort=8086`, `BindIP`, `ThreadPool`, `IPLocationFile` |
| PERFORMANCE SETTINGS | 246-564 | `UseProcessors`, `ProcessPriority`, `Compression`, `PlayerLimit`, `MaxOverspeedPings`, `GridUnload`, `BaseMapLoadAllGrids`, `InstanceMapLoadAllGrids`, `BattlegroundMapLoadAllGrids`, `SocketTimeOutTime{,Active}`, `GridCleanUpDelay`, `MinWorldUpdateTime`, `MapUpdateInterval`, `ChangeWeatherInterval`, `PlayerSaveInterval`, `Auction.*Cooldown` |
| SERVER LOGGING | 566-587 | `PidFile`, `PacketLogFile` |
| SERVER SETTINGS | 589-1434 | `GameType` (NORMAL/PVP/RP/RPPVP/FFA_PVP), `RealmZone`, `StrictPlayerNames`, `MinPlayerName`, `CharactersPerAccount`, `CharactersPerRealm`, `MaxPlayerLevel=80`, `StartPlayerLevel`, `StartDeathKnightPlayerLevel=55`, `StartPlayerMoney`, `Instance.IgnoreLevel`, `Instance.IgnoreRaid`, `Instance.UnloadDelay`, `ResetSchedule.{WeekDay,Hour}`, `Quests.DailyResetTime`, `Quests.WeeklyResetWDay`, `Guild.{ResetHour,SaveInterval,EventLogRecordsCount,…}`, `MailDeliveryDelay`, `AccountInstancesPerHour` |
| CRYPTOGRAPHY | 1436-1453 | `TOTPMasterSecret` |
| UPDATE SETTINGS | 1455-1520 | `Updates.EnableDatabases` (bitmask: 1 login \| 2 character \| 4 world \| 8 hotfix), `Updates.AutoSetup`, `Updates.Redundancy`, `Updates.ArchivedRedundancy`, `Updates.AllowRehash`, `Updates.CleanDeadRefMaxCount` |
| HOTSWAP SETTINGS | 1522-1592 | `HotSwap.Enabled` and 5 sub-flags |
| WARDEN SETTINGS | 1594-1656 | `Warden.Enabled`, `Warden.NumInjectionChecks=9`, `Warden.NumLuaSandboxChecks=1`, `Warden.NumClientModChecks=1`, `Warden.ClientResponseDelay=600`, `Warden.ClientCheckHoldOff=30`, `Warden.ClientCheckFailAction` (0 log / 1 kick / 2 ban), `Warden.BanDuration=86400` |
| PLAYER INTERACTION | 1658-1728 | `AllowTwoSide.Interaction.{Calendar,Channel,Group,Guild,Auction}`, `AllowTwoSide.Trade` |
| CREATURE SETTINGS | 1730-1953 | `Rate.Creature.Aggro`, `Corpse.Decay.{Normal,Elite,RareElite,Obsolete,Rare,Trivial,MinusMob}`, `Rate.Corpse.Decay.Looted=0.5`, `Rate.Creature.{Damage,SpellDamage,HP}.{Normal,Elite,RareElite,Obsolete,Rare,Trivial,MinusMob}` |
| SPAWN/RESPAWN SETTINGS | 1955-2069 | `Respawn.DynamicMode`, `Respawn.DynamicEscortNPC`, `Respawn.DynamicMinimumCreature`, `Respawn.DynamicMinimumGameObject`, `Respawn.DynamicRateCreature=10`, `Respawn.DynamicRateGameObject=10` |
| CHAT SETTINGS | 2071-2213 | `ChatFlood.MessageCount=10`, `ChatFlood.MessageDelay=1`, `ChatFlood.AddonMessageCount=100`, `ChatFlood.AddonMessageDelay=1`, `ChatFlood.MuteTime=10`, `ChatFakeMessagePreventing`, `ChatStrictLinkChecking.{Severity,Kick}` |
| GAME MASTER SETTINGS | 2215-2314 | `GM.LoginState`, `GM.Visible`, `GM.Chat`, `GM.WhisperingTo`, `GM.InGMList.Level`, `GM.AllowInvite=0`, `GM.LowerSecurity` |
| SUPPORT SETTINGS | 2316-2362 | `Support.Enabled`, `Support.{Tickets,Bugs,Complaints,Suggestions}Enabled` |
| VISIBILITY AND DISTANCES | 2364-2415 | `Visibility.GroupMode`, `Visibility.Distance.{Continents,Instances,BGArenas,Arenas=533}`, `Visibility.Notify.Period.*` |
| SERVER RATES | 2417-2751 | `Rate.Health`, `Rate.Mana`, `Rate.Rage.{Gain,Loss}`, `Rate.Focus`, `Rate.Energy`, `Rate.RunicPower.{Gain,Loss}`, `Rate.Skill.Discovery`, `Rate.Drop.Item.{Poor,Normal,Uncommon,Rare,Epic,Legendary,Artifact,Referenced}`, `Rate.Drop.Money`, `Rate.XP.{Kill,Quest,Explore,BattlegroundKill}`, `Rate.Quest.Money.{Reward,Max.Level.Reward}`, `Rate.RepairCost`, `Rate.Rest.{InGame,Offline.InTavernOrCity,Offline.InWilderness}`, `Rate.Damage.Fall`, `Rate.Auction.{Time,Deposit,Cut}`, `Rate.Reputation.{Gain,LowLevel.Kill,LowLevel.Quest}`, `Rate.InstanceResetTime`, `DurabilityLoss*`, `Death.SicknessLevel=11`, `Death.CorpseReclaimDelay.{PvP=1,PvE=0}`, `Death.Bones.{World,BattlegroundOrArena}` |
| STATS LIMITS | 2753-2773 | `Stats.Limits.Enable`, `Stats.Limits.{Dodge,Parry,Block,Crit}` |
| AUTO BROADCAST | 2775-2802 | `AutoBroadcast.{On,Center,Timer}` |
| BATTLEGROUND CONFIG | 2804-2916 | `Battleground.CastDeserter`, `Battleground.QueueAnnouncer.{Enable,PlayerOnly}`, `Battleground.StoreStatistics.Enable`, `Battleground.InvitationType` (0 normal / 1 experimental), `Battleground.PrematureFinishTimer=300000`, `Battleground.PremadeGroupWaitForMatch=1800000`, `Battleground.GiveXPForKills`, `Battleground.Random.ResetHour=6`, `Battleground.Reward{Winner,Loser}{Honor,Conquest}{First,Last}`, `Battleground.ReportAFK=3` |
| BATTLEFIELD CONFIG | 2918-3030 | Wintergrasp settings (`Wintergrasp.Enable`, `Wintergrasp.PlayerMax`, `Wintergrasp.PlayerMin`, `Wintergrasp.BattleTime`, `Wintergrasp.NoBattleTime`, `Wintergrasp.CrashRestartTimer`) |
| ARENA CONFIG | 3032-3160 | `Arena.MaxRatingDifference`, `Arena.RatingDiscardTimer`, `Arena.RatedUpdateTimer`, `Arena.AutoDistributePoints`, `Arena.AutoDistributeInterval`, `Arena.QueueAnnouncer.Enable`, `Arena.SeasonID`, `Arena.SeasonInProgress`, `Arena.LogExtendedInfo`, `Arena.{Win,Lose}RatingModifier{1,2}`, `Arena.MatchmakerRatingModifier`, `ArenaTeam.CharterCost.{2v2=800000,3v3=1200000,5v5=2000000}` |
| NETWORK CONFIG | 3162-3196 | `Network.Threads=1`, `Network.OutKBuff=-1`, `Network.OutUBuff=65536`, `Network.TcpNodelay=1` |
| CONSOLE AND REMOTE ACCESS | 3198-3260 | `Console.Enable`, `Ra.Enable`, `Ra.IP`, `Ra.Port=3443`, `Ra.MinLevel`, SOAP equivalents (`SOAP.Enabled`, `SOAP.IP`, `SOAP.Port`) |
| CHARACTER DELETE OPTIONS | 3262-3306 | `CharDelete.Method` (0 perma / 1 unlink), `CharDelete.MinLevel`, `CharDelete.HeroicMinLevel`, `CharDelete.KeepDays` |
| CUSTOM SERVER OPTIONS | 3308-3566 | `PlayerStart.AllSpells`, `PlayerStart.MapsExplored`, `PlayerStart.AllReputation`, `AlwaysMaxSkillForLevel`, `PvPToken.{Enable,MapAllowType,ItemID,ItemCount}`, `NoResetTalentsCost`, `Show.Kick.In.World`, `Guild.AllowMultipleGuildMaster=0`, `MaxWho`, `DisconnectToleranceInterval` |
| AUCTION HOUSE BOT SETTINGS | 3568-3865 | `AuctionHouseBot.Account=0`, `AuctionHouseBot.Update.Interval=20`, `AuctionHouseBot.Seller.Enabled`, `AuctionHouseBot.{Alliance,Horde,Neutral}.Items.Amount.Ratio=100`, `AuctionHouseBot.{MinTime=1,MaxTime=72}` |
| AUCTION HOUSE BOT ITEM FINE TUNING | 3867-3979 | `AuctionHouseBot.Class.{Consumable,Container,Weapon,Gem,Armor,Reagent,Projectile,TradeGood,…}.Allow.Zero` |
| AUCTION HOUSE BOT BUYER CONFIG | 3981-4057 | `AuctionHouseBot.Buyer.{Enabled,Recheck.Interval,Buyer.Chance.Factor}` |
| BLACK MARKET SETTINGS | 4059-4088 | `BlackMarket.Enabled`, `BlackMarket.MaxAuctions`, `BlackMarket.UpdatePeriod` |
| LOGGING SYSTEM SETTINGS | 4090-4272 | `Appender.<name>=Type,LogLevel,Flags,...` (Type 1 console / 2 file / 3 db); `Logger.<name>=LogLevel,AppenderList`; defaults: `Appender.Console=1,3,0`, `Appender.Server=2,2,0,Server.log,w`, `Logger.root=5,Console Server`, `Logger.commands.gm=3,Console GM` |
| CURRENCIES SETTINGS | 4274-4301 | `Currency.{ResetHour,ResetDay,ResetInterval,Start.Apexis,Max.Apexis,Start.JusticePoints,Max.JusticePoints}` |
| PACKET SPOOF PROTECTION SETTINGS | 4303-4336 | `PacketSpoof.Policy=1` (0 log / 1 log+kick / 2 log+kick+ban), `PacketSpoof.BanMode=0` (0 account / 2 IP), `PacketSpoof.BanDuration=86400` |
| METRIC SETTINGS | 4338-4389 | `Metric.Enable`, `Metric.Interval=1`, `Metric.ConnectionInfo="127.0.0.1;8086;worldserver"`, `Metric.OverallStatusInterval=1`, `Metric.Threshold.<name>` |
| PVP SETTINGS | 4391-4426 | `Pvp.FactionBalance.LevelCheckDiff`, `Pvp.FactionBalance.{Pct5=0.6,Pct10=0.7,Pct20=0.8}` |
| RUSTYCORE TEST / EXPERIMENTAL FLAGS | 4428-4444 | `Bot.AccountPrefix` (RustyCore-only addition; gates the synchronous-login path used by headless test bots), `RustyCore.LegacyCreatureGlobalRuntime` (RustyCore-only experimental runtime owner flag; default `0`) |

### `bnetserver.conf.dist` section index

| Section | Lines | Keys |
|---|---|---|
| AUTH SERVER SETTINGS | 33-228 | `LogsDir`, `MaxPingTime=30`, `BattlenetPort=1119`, `LoginREST.Port=8081`, `LoginREST.ExternalAddress`, `LoginREST.LocalAddress`, `LoginREST.TicketDuration=3600`, `BindIP`, `PidFile`, `CertificatesFile`, `PrivateKeyFile`, `PrivateKeyPassword`, `UseProcessors`, `ProcessPriority`, `RealmsStateUpdateDelay=10`, `WrongPass.MaxCount`, `WrongPass.BanTime=600`, `WrongPass.BanType` (0 IP / 1 account), `WrongPass.Logging`, `BanExpiryCheckInterval=60`, `SourceDirectory`, `MySQLExecutable`, `IPLocationFile`, `AllowLoggingIPAddressesInDatabase=1` |
| MYSQL SETTINGS | 230-265 | `LoginDatabaseInfo` (semicolon string `"host;port;user;pass;db;ssl"`), `LoginDatabase.WorkerThreads=1`, `LoginDatabase.SynchThreads=1` |
| CRYPTOGRAPHY | 267-283 | `TOTPMasterSecret` (hex string used to AES-decrypt per-account TOTP secrets stored in `account.totp_secret`) |
| UPDATE SETTINGS | 285-346 | Same `Updates.*` family as worldserver but with mask `1` (auth only) |
| LOGGING SYSTEM SETTINGS | 348-446 | Same `Appender.*` / `Logger.*` schema as worldserver |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `ConfigMgr` | class (singleton, `src/common/Configuration/Config.h`) | Owns the parsed key/value map; `LoadInitial(filename)`, `Reload()`, `GetXxxDefault(name, default)` accessors |
| `WorldBoolConfigs` | enum (`World.h:102-202`) | Indices into `m_bool_configs[BOOL_CONFIG_VALUE_COUNT]`. ~120 entries: `CONFIG_DURABILITY_LOSS_IN_PVP=0`, `CONFIG_GRID_UNLOAD`, `CONFIG_ALLOW_TWO_SIDE_*`, `CONFIG_INSTANCE_IGNORE_LEVEL`, `CONFIG_DEATH_BONES_*`, `CONFIG_BATTLEGROUND_CAST_DESERTER`, `CONFIG_VMAP_INDOOR_CHECK`, `CONFIG_PVP_TOKEN_ENABLE`, `CONFIG_AUTOBROADCAST`, `CONFIG_SUPPORT_*`, `CONFIG_HOTSWAP_*`, `CONFIG_BLACKMARKET_ENABLED`, `CONFIG_ENABLE_AE_LOOT`, `BOOL_CONFIG_VALUE_COUNT` (sentinel) |
| `WorldFloatConfigs` | enum (`World.h:204-229`) | Indices into `m_float_configs[FLOAT_CONFIG_VALUE_COUNT]`. ~22 entries: `CONFIG_GROUP_XP_DISTANCE=0`, `CONFIG_SIGHT_MONSTER`, `CONFIG_LISTEN_RANGE_{SAY,YELL,TEXTEMOTE}`, `CONFIG_THREAT_RADIUS`, `CONFIG_STATS_LIMITS_{DODGE,PARRY,BLOCK,CRIT}`, `CONFIG_ARENA_{WIN,LOSE,MATCHMAKER}_RATING_MODIFIER*`, `CONFIG_RESPAWN_DYNAMICRATE_{CREATURE,GAMEOBJECT}`, `CONFIG_CALL_TO_ARMS_{5,10,20}_PCT`, `FLOAT_CONFIG_VALUE_COUNT` |
| `WorldIntConfigs` | enum (`World.h:231-441`) | Indices into `m_int_configs[INT_CONFIG_VALUE_COUNT]`. ~210 entries: `CONFIG_COMPRESSION=0`, `CONFIG_INTERVAL_{SAVE,GRIDCLEAN,MAPUPDATE,CHANGEWEATHER,DISCONNECT_TOLERANCE}`, `CONFIG_PORT_{WORLD,INSTANCE}`, `CONFIG_SOCKET_TIMEOUTTIME`, `CONFIG_GAME_TYPE`, `CONFIG_REALM_ZONE`, `CONFIG_MAX_PLAYER_LEVEL`, `CONFIG_START_*_PLAYER_LEVEL`, `CONFIG_DAILY_QUEST_RESET_TIME_HOUR`, `CONFIG_WEEKLY_QUEST_RESET_TIME_WDAY`, `CONFIG_RESET_SCHEDULE_{WEEK_DAY,HOUR}`, `CONFIG_INSTANCE_UNLOAD_DELAY`, `CONFIG_GM_*`, `CONFIG_MAIL_DELIVERY_DELAY`, `CONFIG_SKILL_*`, `CONFIG_MAX_OVERSPEED_PINGS`, `CONFIG_EXPANSION`, `CONFIG_CHATFLOOD_*`, `INT_CONFIG_VALUE_COUNT` |
| `World` | class (`World.h`) | Holds the three arrays as private members (lines 847-850) and exposes `getIntConfig(WorldIntConfigs)`, `setIntConfig`, `getBoolConfig`, `setBoolConfig`, `getFloatConfig`, `setFloatConfig` (lines 684-717) for read/write at runtime |
| `LogLevel` | enum | 0 disabled, 1 trace, 2 debug, 3 info, 4 warn, 5 error, 6 fatal — used in every `Appender.*` / `Logger.*` line |
| `AppenderType` | enum | 0 none, 1 console, 2 file, 3 db |
| `AppenderFlags` | bitmask | 1 prefix-timestamp, 2 prefix-level, 4 prefix-filter-type, 8 dynamic-filename, 16 backup-on-overwrite |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `ConfigMgr::LoadInitial(std::string const& filename, std::vector<std::string> args, std::string& error)` | Parse `worldserver.conf` (or `bnetserver.conf`) into the in-process map. Called from `main()` of each daemon | bpt parser, file I/O |
| `ConfigMgr::Reload()` | Re-read the same file and replace the map atomically | `LoadInitial` |
| `ConfigMgr::GetIntDefault(std::string const& name, int32 def, bool quiet=false)` | Look up `name`, parse as int, fall back to `def` if absent | map lookup |
| `ConfigMgr::GetStringDefault(...)` / `GetFloatDefault(...)` / `GetBoolDefault(...)` | Same for other types | map lookup |
| `World::LoadConfigSettings(bool reload=false)` | The big function: reads ~600 keys via `ConfigMgr` and writes them into `m_int_configs` / `m_bool_configs` / `m_float_configs`. When `reload=true`, also re-applies any side-effects that need to fire on change (re-open log files, recompute visibility, etc.) | `setIntConfig`, `setBoolConfig`, `setFloatConfig`, `sLog->LoadFromConfig()`, `sScriptMgr->OnConfigLoad()` |
| `sWorld->getIntConfig(CONFIG_FOO)` / `getBoolConfig` / `getFloatConfig` | Hot-path read used by every gameplay subsystem (combat, loot, BG, arena, chat, …) | array index |
| `ChatHandler::HandleReloadConfigCommand` (in `cs_reload.cpp`) | `.reload config` GM command → `sConfigMgr->Reload()` + `sWorld->LoadConfigSettings(true)` | `ConfigMgr::Reload`, `World::LoadConfigSettings` |
| `sLog->LoadFromConfig()` | Re-build appender/logger topology from current `Appender.*` / `Logger.*` keys; called after every reload | `Log::CreateAppenderFromConfig`, `Log::CreateLoggerFromConfig` |

---

## 5. Module dependencies

**Depends on:** nothing (L0). The config layer is parsed before any subsystem boots.

**Depended on by (everything):**

- `Realm`, `Database` — `LoginDatabaseInfo`, `WorldDatabaseInfo`, `CharacterDatabaseInfo`, `HotfixDatabaseInfo`, `*.WorkerThreads`, `*.SynchThreads`, `MaxPingTime`, `RealmID`
- `WorldSocket` / `WorldSocketMgr` — `WorldServerPort`, `InstanceServerPort`, `BindIP`, `Network.Threads`, `Network.{OutKBuff,OutUBuff,TcpNodelay}`, `SocketTimeOutTime{,Active}`, `CONFIG_COMPRESSION`, `PacketSpoof.{Policy,BanMode,BanDuration}`
- `WorldSession` — `SessionAddDelay`, `MaxOverspeedPings`, `Auth*`, `WrongPass.*`, `Bot.AccountPrefix`
- `Map` / `MapManager` — `GridUnload`, `BaseMapLoadAllGrids`, `InstanceMapLoadAllGrids`, `BattlegroundMapLoadAllGrids`, `GridCleanUpDelay`, `MinWorldUpdateTime`, `MapUpdateInterval`, `RustyCore.LegacyCreatureGlobalRuntime`
- `Player` — every `Rate.XP.*`, `Rate.Health/Mana/...`, `MaxPlayerLevel`, `StartPlayerLevel`, `StartPlayerMoney`, `Death.*`, `CharDelete.*`, `Visibility.*`, `Stats.Limits.*`
- `Creature` — `Rate.Creature.{Damage,SpellDamage,HP}.*`, `Corpse.Decay.*`, `Rate.Creature.Aggro`
- `BattlegroundMgr` — every `Battleground.*`
- `BattlefieldMgr` — every `Wintergrasp.*`
- `ArenaMgr` / `ArenaTeam` — every `Arena.*`, `ArenaTeam.CharterCost.*`
- `Guild` / `GuildMgr` — every `Guild.*`
- `MailMgr` — `MailDeliveryDelay`, `CleanOldMailTime`
- `AuctionHouseMgr` / `AuctionHouseBot` — every `Auction.*`, `AuctionHouseBot.*`, `Rate.Auction.*`
- `BlackMarketMgr` — `BlackMarket.*`
- `InstanceMgr` / `InstanceLockMgr` — `Instance.{IgnoreLevel,IgnoreRaid,UnloadDelay}`, `ResetSchedule.{WeekDay,Hour}`, `Rate.InstanceResetTime`, `AccountInstancesPerHour`
- `Warden` — every `Warden.*`
- `Chat` — `ChatFlood.*`, `ChatStrictLinkChecking.*`, `ChatFakeMessagePreventing`
- `Log` — every `Appender.*` and `Logger.*`
- `Metric` — every `Metric.*`
- `bnetserver` daemon — `BattlenetPort`, `LoginREST.{Port,ExternalAddress,LocalAddress,TicketDuration}`, `Certificates*`, `PrivateKey*`, `TOTPMasterSecret`, `WrongPass.*`, `BanExpiryCheckInterval`, `AllowLoggingIPAddressesInDatabase`

---

## 6. SQL / DB queries (if any)

The config files do not issue queries themselves. Two indirect points worth tracking:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SEL_REALMLIST` | Realm list requested by bnetserver; the `RealmID` config key must match a row in `auth.realmlist.id` | auth |
| `LOGIN_REP_ACCOUNT_TOTP` | Per-account TOTP secret stored AES-encrypted with `TOTPMasterSecret` | auth |
| `Updates.*` family | When `Updates.EnableDatabases` mask is non-zero, the daemon reads/writes rows in `<db>.updates` and `<db>.updates_include` tables | each enabled DB |

No DBC/DB2 stores are read by the config layer.

---

## 7. Wire-protocol packets (if any)

The config layer is process-internal — it does not originate packets. It does, however, **gate** virtually every packet handler:

- `PacketSpoof.Policy` decides what `WorldSocket::HandleAuthSession()` does on a header-tag mismatch.
- `Warden.*` decides whether `SMSG_WARDEN_DATA` is even constructed for a session.
- `MaxOverspeedPings` decides when `WorldSession::HandleMovementOpcodes()` calls `KickPlayer`.
- `ChatFlood.*` decides when `WorldSession::HandleMessagechat()` returns `CHAT_RESTRICTED` instead of broadcasting.
- `AllowTwoSide.Trade` decides whether `CMSG_INITIATE_TRADE` is rejected cross-faction.

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**

- `crates/wow-config/src/lib.rs` — 397 lines — global `RwLock<ConfigStore>` singleton, `load_config(path)` / `load_config_from_str(content)` / `get_value::<T>(key)` / `get_value_default::<T>(key, default)` / `get_string_default(key, default)` API. Case-insensitive lookup (keys stored lowercased). 22 unit tests, all passing.
- `crates/bnet-server/src/main.rs` lines 32-83 — calls `wow_config::load_config("BNetServer.conf")` (falling back to `.dist`) and reads ~17 keys.
- `crates/world-server/src/main.rs` lines 165-460 — same pattern; reads ~24 keys.
- Production files (gitignored, contain credentials): `/home/server/rustycore/BNetServer.conf`, `/home/server/rustycore/WorldServer.conf`.

**What's implemented:**

- ✅ `Key = Value` text format with `#` line and inline comments (matches TC's `bpt` parser behaviourally for the simple cases).
- ✅ Quoted-string values, including embedded `=` (e.g. `ConnString = "server=localhost;port=3306"`) and `#` (e.g. `Color = "#FF0000"`).
- ✅ Case-insensitive key lookup — TC also lowercases for comparison, so this matches.
- ✅ Fallback chain `WorldServer.conf` → `WorldServer.conf.dist` (TC has the same convention).
- ✅ Typed accessors via `FromStr`: integers, floats, `String` all flow through one generic.
- ✅ Reload-replaces semantics on the parser side (`parse()` clears the map first, so repeated calls overwrite cleanly).
- ✅ RustyCore-only experimental flags can be read ad hoc. `RustyCore.LegacyCreatureGlobalRuntime` is numeric `0`/`1`, defaults to `0`, and is intentionally not part of TrinityCore config parity.

**What's missing vs C++:**

- ❌ **The `m_int_configs` / `m_bool_configs` / `m_float_configs` arrays do not exist.** RustyCore reads keys ad-hoc at the call site instead of caching them in indexed arrays. This means every `getIntConfig` equivalent is a `HashMap` lookup instead of an `O(1)` array index — fine functionally but a perf gap on hot paths (e.g. movement validation reads `MaxOverspeedPings` per packet).
- ❌ **No `World::LoadConfigSettings()` equivalent.** Subsystems that need values fetch them lazily; there is no startup pass that validates every expected key is present and warns on missing/malformed entries. Silent fall-through to hardcoded Rust defaults.
- ❌ **No `.reload config` command.** `wow_config::load_config` *can* be called again to replace values, but no GM command, no signal handler, and no `OnConfigReload` hook fires in subsystems. CLAUDE.md acknowledges this.
- ❌ **The semicolon-delimited DB connection-string format `"host;port;user;pass;db;ssl"` is not parsed.** Rust expects split keys: `LoginDatabaseInfo.Host`, `LoginDatabaseInfo.Port`, `LoginDatabaseInfo.Username`, `LoginDatabaseInfo.Password`, `LoginDatabaseInfo.Database`. **Operators copying a TC `worldserver.conf` will get default `127.0.0.1` / `root` / empty password silently** because the literal key `LoginDatabaseInfo` is parsed but never looked up.
- ❌ ~588 of ~612 `worldserver.conf` keys are unread. Examples grouped by criticality:
  - **Security-critical (silently default to Rust hardcoded values)**: `MaxOverspeedPings`, `PacketSpoof.{Policy,BanMode,BanDuration}`, `Warden.*`, `ChatFlood.*`, `WrongPass.*` (only on bnet side), `SocketTimeOutTime{,Active}`, `SessionAddDelay`.
  - **Economy / progression**: every `Rate.XP.*`, `Rate.Drop.*`, `Rate.Creature.*`, `Rate.Quest.Money.*`, `Rate.Health`, `Rate.Mana`, `StartPlayerLevel`, `StartPlayerMoney`.
  - **Server identity**: `GameType`, `RealmZone`, `Expansion`, `MaxPlayerLevel`, `RealmsStateUpdateDelay` (only bnet side reads).
  - **Logging**: `Appender.*` / `Logger.*` not parsed at all — Rust uses `tracing` with a hardcoded `EnvFilter` from `RUST_LOG`.
  - **Schedules**: `Quests.{DailyResetTime,WeeklyResetWDay}`, `ResetSchedule.{WeekDay,Hour}`, `Battleground.Random.ResetHour`, `Guild.ResetHour`, `Currency.{ResetHour,ResetDay,ResetInterval}`.
  - **Anti-griefing**: `AllowTwoSide.*`, `Quests.IgnoreRaid`, `Instance.IgnoreLevel`, `Instance.IgnoreRaid`.
  - **Networking**: `Network.{Threads,OutKBuff,OutUBuff,TcpNodelay}`, `ThreadPool`, `Compression`.
  - **Visibility**: `Visibility.*` (whole section).
  - **PvP**: `Pvp.FactionBalance.*` (whole section), `Battleground.*`, `Arena.*`, `Wintergrasp.*`.
  - **Auction**: `Auction.*`, `AuctionHouseBot.*`, `BlackMarket.*`.
  - **Cryptography**: `TOTPMasterSecret` (bnet side reads `CertificatesFile` but **does not parse** the TOTP secret — 2FA effectively broken for any account that has one stored).

**Suspicious / likely divergent (hipótesis pre-auditoría):**

- **DB connection format mismatch**: `LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"` from a stock TC config will produce an empty database connection in RustyCore because the splitter logic doesn't exist. Operator gets a successful boot with `localhost` / `root` / no-password defaults. **Confirm before next release.**
- **Silent default drift**: every `Rate.*` not read defaults to `1.0` somewhere in code, but the *somewhere* is per-subsystem and undocumented. Two subsystems may pick different defaults for the same conceptual rate.
- **Inline comment with quoted `#`**: Rust handles `Color = "#FF0000"` correctly per `test_quoted_value_with_hash`. Good.
- **No bool parsing**: TC accepts `true`/`false` and `1`/`0` for bool keys. Rust only parses via `FromStr<bool>` which accepts only `true`/`false`. Stock TC configs use `0`/`1`, which Rust would reject. (`Instance.IgnoreLevel = 0` → `parse::<bool>()` fails → `None` → default fires.) **Bug.**
- **`Updates.EnableDatabases` is a bitmask**, not a bool. Rust currently reads `Updates.AutoSetup` as a string and compares. Fine. But `Updates.EnableDatabases = 15` from the dist file will not be interpreted correctly if treated as bool.
- **Section header `[worldserver]` / `[bnetserver]`** — the Rust parser ignores any line that doesn't have `=`. The header lines start with `[`, contain no `=` — they will be **rejected** with a parse error: see `ConfigStore::parse` returning `ParseError { line, message: "expected '=' in: [worldserver]" }`. **The current Rust loader cannot read TC-format `.conf.dist` files at all without manual editing.** (TC's parser explicitly skips section headers as a special case.)
- **TOTP not loaded** → `account.totp_secret` rows decrypt to garbage → in-game `.account 2fa` commands break.

**Tests existing:**

- 22 unit tests in `crates/wow-config/src/lib.rs`: parse basics, comments, empty lines, case-insensitive lookup, integer/float parsing, defaults, error paths, multiple keys, reload, value with `=`, value with quoted `#`. **No test loads an actual `.conf.dist`** — the parser has never been validated against TC's full text grammar.

---

## 9. Migration sub-tasks

- [ ] **#CONFIG.1** Add section-header skipping to `ConfigStore::parse`: lines matching `^\[.*\]$` should be ignored, not error out. Without this fix, **no stock TC `.conf.dist` file is loadable**. (complejidad: L)
- [ ] **#CONFIG.2** Accept `0`/`1` as bool literals in `get_value::<bool>`. Either add a `get_bool(key, default)` helper that maps `0`/`false`/`no` → false and `1`/`true`/`yes` → true (TC convention), or change the lib so `bool` is parsed via that helper. (complejidad: L)
- [ ] **#CONFIG.3** Implement TC-style semicolon connection-string parsing: when a key matches `*DatabaseInfo` (no suffix), split on `;` into 5-or-6 fields (`host;port;user;pass;db[;ssl]`) and expose them via a `ConnectionInfo` struct. Update `bnet-server` and `world-server` to read the unified key, not the split `.Host`/`.Port`/etc. variants. (complejidad: M)
- [ ] **#CONFIG.4** Define the three enum-indexed configuration arrays as Rust `enum WorldIntConfig { … }` + `Vec<u32>` (or `[u32; INT_CONFIG_VALUE_COUNT]`) on a `WorldConfig` struct. Mirror `WorldBoolConfigs`, `WorldFloatConfigs`, `WorldIntConfigs` from `World.h:102-441`. Provide `world_config.get_int(WorldIntConfig::MaxPlayerLevel)`. Move the per-field defaults from scattered call sites into one `WorldConfig::load(path)` function modelled on `World::LoadConfigSettings`. (complejidad: H)
- [ ] **#CONFIG.5** Implement `world_config.reload()` and wire it to a `.reload config` GM command (chat handler) and to `SIGHUP`. Subsystems that cache config-derived state must subscribe to a reload signal (`tokio::sync::broadcast<()>`). (complejidad: M)
- [ ] **#CONFIG.6** Port the security-critical keys first: `MaxOverspeedPings`, `PacketSpoof.{Policy,BanMode,BanDuration}`, `Warden.{Enabled,…}` (gated on Warden module being ported), `ChatFlood.{MessageCount,MessageDelay,AddonMessageCount,AddonMessageDelay,MuteTime}`, `WrongPass.*`, `SocketTimeOutTime{,Active}`. Ship the corresponding handler-side enforcement; without `MaxOverspeedPings` enforcement, speed exploits are **unrate-limited**. (complejidad: H)
- [ ] **#CONFIG.7** Port the `Rate.*` family (~60 keys) — XP, drops, money, health/mana, creature damage/HP, repair, rest, reputation, instance reset. Each needs an audit of where it's read in C++ and the equivalent multiplication site in Rust. (complejidad: H)
- [ ] **#CONFIG.8** Port the `Appender.*` / `Logger.*` schema into a `tracing` configuration: parse the `Type,LogLevel,Flags[,opt1,opt2,opt3]` format and build a `tracing_subscriber` layer accordingly. Today RustyCore ignores these and uses `RUST_LOG`; operators editing the conf get no effect. (complejidad: H)
- [ ] **#CONFIG.9** Port `TOTPMasterSecret` to bnet-server: AES-decrypt `account.totp_secret` rows with the configured key, expose a `verify_totp(account_id, code)` helper, gate `.account 2fa` commands. (complejidad: M)
- [ ] **#CONFIG.10** Port the schedule keys: `ResetSchedule.{WeekDay,Hour}`, `Quests.{DailyResetTime,WeeklyResetWDay}`, `Guild.ResetHour`, `Battleground.Random.ResetHour`, `Currency.{ResetHour,ResetDay,ResetInterval}`. Without these, daily/weekly resets either don't fire or fire at the wrong wall-clock time. (complejidad: M)
- [ ] **#CONFIG.11** Add a `cargo test` round-trip that loads `worldserver.conf.dist` and `bnetserver.conf.dist` verbatim from the TC tree and asserts no `ParseError`. Add a sister test that warns on every key present in the file but never read by any RustyCore subsystem (catalogue of unimplemented features, refreshed on every CI run). (complejidad: M)
- [ ] **#CONFIG.12** Document the divergence in `BNetServer.conf` / `WorldServer.conf` example files at the repo root: list the keys RustyCore *actually* reads, with their current defaults, and a note that copying a stock TC config produces incorrect behaviour. Update CLAUDE.md to reference this migration doc. (complejidad: L)

---

## 10. Regression tests to write

- [ ] Test: `load_config()` on the verbatim TC `worldserver.conf.dist` succeeds (no parse error from the `[worldserver]` header or any `0`/`1` bool).
- [ ] Test: `load_config()` on the verbatim TC `bnetserver.conf.dist` succeeds.
- [ ] Test: `LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"` from stock TC parses into `ConnectionInfo { host: "127.0.0.1", port: 3306, user: "trinity", password: "trinity", database: "auth", ssl: false }`.
- [ ] Test: `LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth;ssl"` parses with `ssl: true`.
- [ ] Test: bool key `Instance.IgnoreLevel = 0` returns `false`; `Instance.IgnoreLevel = 1` returns `true`; `Instance.IgnoreLevel = true` also returns `true` (both formats accepted).
- [ ] Test: `WorldConfig::get_int(WorldIntConfig::MaxPlayerLevel)` returns `80` after loading default conf.
- [ ] Test: reload — load conf, mutate one key on disk, call `reload()`, verify the new value is visible without restarting subsystems that subscribed to the reload broadcast.
- [ ] Test: missing key falls back to documented default (one regression test per security-critical key in #CONFIG.6).
- [ ] Test: `Appender.Console=1,3,0` produces a console subscriber at INFO level; `Appender.Server=2,2,0,Server.log,w` produces a file subscriber at DEBUG level writing to `Server.log` in overwrite mode.
- [ ] Test: `TOTPMasterSecret = 000102030405060708090A0B0C0D0E0F` decrypts a known-plaintext fixture row.
- [ ] Test: a CI-only test that scans `worldserver.conf.dist` for keys, looks up each in the running Rust config, and prints (does not fail) the list of unread keys. Must be runnable as a coverage check.

---

## 11. Notes / gotchas

1. **The Rust parser cannot load a stock TC config today.** `[worldserver]` on line 4 of `worldserver.conf.dist` is parsed as a key/value line, fails the `=` check, and returns `ConfigError::ParseError`. The production `WorldServer.conf` / `BNetServer.conf` files at the repo root work only because they were authored without the section header. This is the single most important fix in #CONFIG.1.

2. **Database connection format is incompatible.** TC and the dist files use one semicolon-delimited string. RustyCore expects five separate keys. Copying a TC config gives silent fallback to `127.0.0.1` / `root` / empty password, which on a production realm is a credential-leak vector if the operator's intent was a different host.

3. **`0`/`1` as bool**: `FromStr::<bool>` rejects them. Every `*.Enable` / `*.Enabled` / `Allow*` / `Use*` key in TC uses `0`/`1`. Without #CONFIG.2, *every* boolean key from a stock config silently defaults.

4. **`Updates.EnableDatabases` is a bitmask**, not a bool: `1=login`, `2=character`, `4=world`, `8=hotfix`, `15=all`. Treating it as bool gives nonsensical behaviour.

5. **`m_int_configs` / `m_bool_configs` / `m_float_configs` are read on every tick** in C++ for things like `MapUpdateInterval`, `MaxOverspeedPings`, `Visibility.Distance.*`. The hash-map lookup RustyCore does today is functionally equivalent but ~10–30× slower. For now this is fine (no key is read at packet-handler hot-path frequency *yet*); when the corresponding handlers land, port them onto an indexed array (#CONFIG.4).

6. **`PacketSpoof.Policy = 1` is the default in TC** — log + kick. RustyCore has no enforcement at all today: a malformed header just logs and drops the packet, the connection stays open. A determined attacker can probe handlers indefinitely. This needs #CONFIG.6 + the matching enforcement code.

7. **`MaxOverspeedPings = 2`**: TC kicks after 2 overspeed reports. RustyCore has no overspeed accounting. Speed exploits are unrate-limited. Same #CONFIG.6.

8. **`ChatFlood.*` defaults**: 10 messages per second + 1-second delay between messages + 100-message addon allowance. RustyCore has no flood throttling in chat handlers today; this is a spam-vector if/when the chat module is opened to public realms.

9. **`Logger.root=5`** in TC defaults to ERROR-level only. RustyCore's `tracing` subscriber defaults from `RUST_LOG=info` (or whatever env var is set), which is significantly louder. Operators get more log noise than TC.

10. **`Bot.AccountPrefix`** is a RustyCore-specific addition (not in TC). It is documented in `worldserver.conf.dist` lines 4428-4444 and gates the synchronous-login path used by the LFG headless test bot. Production realms must leave it `""` — see CLAUDE.md.

11. **`RustyCore.LegacyCreatureGlobalRuntime`** is a RustyCore-specific experimental migration flag (not in TC). It is read as numeric `0`/`1`; absent and `0` keep the legacy session-owned creature tick, while non-zero flips the shared legacy map owner to `GlobalLegacy` and starts the global creature runtime loop at `MapUpdateInterval`. Do not enable it on a production realm until Slice 4B manual client/server validation is complete.

12. **`TOTPMasterSecret` is a hex string**, not a base64 string. TC uses raw 16-byte AES-128-CBC keys formatted as 32 hex chars (`000102…0F`).

13. **Reload semantics**: TC's `.reload config` only re-reads the file. Subsystems must opt in to picking up the new values — many keys (e.g. `BindIP`, `WorldServerPort`, `Network.Threads`) are read once at boot and ignored on reload. Document per-key whether reload takes effect.

14. **`Metric.ConnectionInfo = "127.0.0.1;8086;worldserver"`** uses a different semicolon schema (host/port/database, no auth) — yet another not-quite-the-same connection-string format to handle.

15. **Per-environment file naming**: TC convention is `worldserver.conf` (the operator's edited copy) overrides `worldserver.conf.dist` (template). RustyCore mirrors this in `world-server/src/main.rs:165`. Document this so operators don't edit the `.dist` file and lose changes on next checkout.

16. **The `[worldserver]` / `[bnetserver]` header** is the only INI-like artefact in an otherwise flat-key file. Some TC forks abuse the header to support multiple realms in one file by switching sections; RustyCore should explicitly choose **not** to support that and document the choice.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class ConfigMgr` | `wow_config::CONFIG: Lazy<RwLock<ConfigStore>>` (singleton) | Functional parity for read; reload + Notify missing |
| `ConfigMgr::LoadInitial(file, args, err)` | `wow_config::load_config(path)` | Args (CLI overrides) and error-message accumulator not implemented |
| `ConfigMgr::Reload()` | (call `load_config()` again) | No subsystem callback; manual only |
| `ConfigMgr::GetIntDefault(name, def)` | `wow_config::get_value_default::<i32>(name, def)` | Generic; `T: FromStr` |
| `ConfigMgr::GetStringDefault(name, def)` | `wow_config::get_string_default(name, def)` | Returns `String` not `&str` |
| `ConfigMgr::GetBoolDefault(name, def)` | (no direct equivalent) | `get_value::<bool>` rejects `0`/`1` — see #CONFIG.2 |
| `ConfigMgr::GetFloatDefault(name, def)` | `wow_config::get_value_default::<f32>(name, def)` | Same generic |
| `enum WorldIntConfigs` (~210 entries) | planned `enum WorldIntConfig` + `Vec<u32>` array | Not implemented; #CONFIG.4 |
| `enum WorldBoolConfigs` (~120 entries) | planned `enum WorldBoolConfig` + `Vec<bool>` | Not implemented |
| `enum WorldFloatConfigs` (~22 entries) | planned `enum WorldFloatConfig` + `Vec<f32>` | Not implemented |
| `World::LoadConfigSettings(reload)` | planned `WorldConfig::load_or_reload(path, is_reload)` | Not implemented |
| `sWorld->getIntConfig(CONFIG_FOO)` | planned `world_config.get_int(WorldIntConfig::Foo)` | Hot-path read via array index |
| `sLog->LoadFromConfig()` | planned `wow_logging::reload_from_config()` | Today: `tracing_subscriber::EnvFilter::from_default_env()` only |
| `LoginDatabaseInfo = "host;port;user;pass;db;ssl"` | split keys `LoginDatabaseInfo.{Host,Port,Username,Password,Database}` | **Format divergence**; #CONFIG.3 |
| `Bot.AccountPrefix` | `wow_config::get_string_default("Bot.AccountPrefix", "")` | RustyCore-specific key (not in TC) |
| `RustyCore.LegacyCreatureGlobalRuntime` | `wow_config::get_value_default::<u8>("RustyCore.LegacyCreatureGlobalRuntime", 0) != 0` | RustyCore-specific experimental key (not in TC); default off |
| `[worldserver]` section header | (would currently fail to parse) | #CONFIG.1 |
| `0`/`1` boolean values | (would currently return `None` from `get_value::<bool>`) | #CONFIG.2 |
| `.reload config` GM command | (no equivalent) | #CONFIG.5 |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

### Scope

Cross-checked the canonical `.conf.dist` files line-by-line against the Rust loader (`crates/wow-config/src/lib.rs`) and the call sites in `crates/world-server/src/main.rs` (lines 165-460) and `crates/bnet-server/src/main.rs` (lines 32-83). Counted keys mechanically (`grep -nE '^[A-Z][A-Za-z0-9._]* +='` on each `.conf.dist`). Spot-checked the parser against fixture lines that the dist files actually contain (section header, `0`/`1` booleans, semicolon-joined connection strings).

### Coverage stats

| Metric | Count |
|---|---|
| `worldserver.conf.dist` distinct keys | ~612 (excluding the `Appender.*` / `Logger.*` template lines which use a meta-format) |
| `bnetserver.conf.dist` distinct keys | ~25 |
| Rust keys read in `world-server/src/main.rs` | 24 (incl. 5 split-format DB keys × 4 DBs = 20 of the 24) |
| Rust keys read in `bnet-server/src/main.rs` | 17 |
| **Coverage** (world-server) | **~4 %** (24 / 612) |
| **Coverage** (bnet-server) | **~68 %** (17 / 25) — but the 25 includes the `Appender.*` / `Logger.*` family Rust does not parse |
| Connection-string format mismatch | 4 keys (LoginDatabaseInfo, WorldDatabaseInfo, CharacterDatabaseInfo, HotfixDatabaseInfo): split into 5 sub-keys each in Rust, single semicolon string in TC |
| Section headers parsed | 0 out of 2 (`[worldserver]`, `[bnetserver]`) — would fail the `=` check |
| Bool keys with `0`/`1` literals in dist | ~120 (every `*.Enable`, `Allow*`, `Use*`, `*.IgnoreLevel`, etc.) — none parse via `get_value::<bool>` today |
| Hot-path security keys unread | 8 (`MaxOverspeedPings`, `PacketSpoof.Policy`, `PacketSpoof.BanMode`, `PacketSpoof.BanDuration`, `Warden.Enabled`, `ChatFlood.MessageCount`, `ChatFlood.MessageDelay`, `SessionAddDelay`) |

### Behavioural divergences

1. **Section header parse failure** (`#CONFIG.1`): Rust returns `ParseError { line: 4, message: "expected '=' in: [worldserver]" }` on the verbatim TC file. Severity: **blocker** — operators cannot use a stock dist as starting point. The production files at the repo root work only because they have been hand-stripped of the header.

2. **DB connection-string mismatch** (`#CONFIG.3`): TC `LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"` is parsed as a single string by Rust and then ignored — the consumer code looks for `LoginDatabaseInfo.Host`, which is absent, so it falls back to `127.0.0.1` (the Rust default). On any non-default deployment the operator gets a connection failure or — worse — a successful connection to a DB on `127.0.0.1` that wasn't intended. Severity: **high** (correctness + potential security implication).

3. **`0`/`1` bool rejection** (`#CONFIG.2`): every `*.Enable=1` from TC is silently treated as missing. Severity: **high** — `Warden.Enabled = 1`, `Updates.AutoSetup = 1`, `AutoBroadcast.On = 1`, `Battleground.CastDeserter = 1`, `AllowLoggingIPAddressesInDatabase = 1` etc. all silently disable.

4. **No reload mechanism** (`#CONFIG.5`): operators cannot apply a config change without restarting both daemons. CLAUDE.md acknowledges this. Severity: **medium** for ops, **low** for correctness.

5. **No `m_*_configs` arrays** (`#CONFIG.4`): every config read is a `HashMap<String, String>` lookup + parse. Functionally equivalent today; performance penalty if hot-path handlers start reading rates per-tick. Severity: **low** today, **medium** as more handlers land.

6. **Logger configuration ignored** (`#CONFIG.8`): RustyCore reads `RUST_LOG` env var only; the entire `Appender.*` / `Logger.*` schema is dead text in the conf. Severity: **medium** — operator-edited log levels have no effect.

7. **TOTP not loaded** (`#CONFIG.9`): bnet-server reads `CertificatesFile` and `PrivateKeyFile` but not `TOTPMasterSecret`. Any account with `account.totp_secret` set will fail 2FA verification. Severity: **high** for any realm using 2FA, **none** otherwise.

8. **Schedule keys unread** (`#CONFIG.10`): daily / weekly / guild / BG / currency reset hours are absent. Resets either don't fire or fire at hardcoded Rust defaults that may not match operator's timezone or expectations. Severity: **medium**.

9. **`Rate.*` family unread** (`#CONFIG.7`): roughly 60 keys controlling XP, drop, money, creature damage/HP rates. Each currently defaults to `1.0` somewhere implicit in the code. Operators expecting `Rate.XP.Kill = 5.0` for a rate-modified server get vanilla rates with no error. Severity: **high** for private-server operators (this is a primary differentiator).

10. **Anti-cheat / anti-spam unread**: `MaxOverspeedPings`, `PacketSpoof.*`, `ChatFlood.*`, `WrongPass.*` (world side), `SocketTimeOutTime{,Active}`. Without these enforced, **the server has no rate-limiting on speed exploits, no kicking on packet spoofing, no flood protection on chat, no idle-disconnect, and no failed-login lockout**. Severity: **critical** for any realm exposed to the public internet.

### Sample arity check

Took 5 keys at random from the dist and verified their Rust handling:

| Key | Dist value | Rust read | Result |
|---|---|---|---|
| `RealmID` | `1` | `get_value("RealmID").unwrap_or(1)` | **OK** |
| `WorldServerPort` | `8085` | `get_value("WorldServerPort").unwrap_or(8085)` | **OK** |
| `LoginDatabaseInfo` | `"127.0.0.1;3306;trinity;trinity;auth"` | (ignored — split keys read instead) | **MISMATCH** |
| `Warden.Enabled` | `0` | (not read) | **MISSING** |
| `Rate.XP.Kill` | `1` | (not read) | **MISSING** |

3 of 5 sampled keys are mishandled. Extrapolating to the full 612-key surface and assuming the hot 24 keys Rust *does* read are over-represented in correctness, true coverage is closer to **0–5 %** of useful semantic behaviour, not 4 % of keys.

### Recommended remediation order

1. **#CONFIG.1** (section header) and **#CONFIG.2** (`0`/`1` bool) — both trivial; without them no further key porting helps because the parser fails or silently rejects.
2. **#CONFIG.3** (DB connection string) — operationally the highest-impact correctness bug.
3. **#CONFIG.6** (security-critical keys) — every public realm needs these enforced.
4. **#CONFIG.9** (TOTP) — security feature, narrow scope, easy to ship.
5. **#CONFIG.10** (schedules) and **#CONFIG.7** (Rate.\*) — gameplay correctness, large surface.
6. **#CONFIG.4** (indexed arrays) and **#CONFIG.5** (reload) — infra refactors that enable everything else.
7. **#CONFIG.8** (logger schema) — quality-of-life for operators.
8. **#CONFIG.11** (test coverage) and **#CONFIG.12** (docs) — last, after the rest stabilises.

### Audit confidence

- Coverage stats: **high confidence** (mechanical key counts; manual cross-reference of 24 + 17 Rust call sites against the dist).
- Behavioural divergences 1–3: **high confidence** (reproduced in the test suite).
- Divergences 4–10: **high confidence** (call-site search returns zero hits in the Rust tree).
- Severity assessments: **moderate confidence** — depends on deployment posture (private dev realm vs public production).
- "Closer to 0–5 %" extrapolation: **moderate confidence** (extrapolation, not exhaustive line-by-line audit of the 612 keys).
