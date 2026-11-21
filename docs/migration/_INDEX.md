# Historical C++ subsystem reference

These source locators come from the archived WotLK migration reports. Follow the [current checkpoint](STATE.md) and [master plan](PORT_PLAN.md) for implementation status. Each report opens at the revision that records its findings.

For offline access use `git show 5fe18d391a4adb43e72a764f9235d855761fdf24:docs/migration/<filename>.md`. The historical TSV inventories retain the report filenames from that revision.

| Subsystem report | C++ source locator in that report |
| --- | --- |
| [accounts](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/accounts.md) | `src/server/game/Accounts/` |
| [achievements](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/achievements.md) | `/home/server/woltk-trinity-legacy/src/server/game/Achievements/`, `src/server/game/Handlers/AchievementHandler.cpp`, `src/server/game/Server/Packets/AchievementPackets.{h,cpp}` |
| [ai base](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/ai-base.md) | `src/server/game/AI/`, `CoreAI/`, `ScriptedAI/ScriptedCreature.{h,cpp}` |
| [ai smartscripts](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/ai-smartscripts.md) | `src/server/game/AI/SmartScripts/` |
| [ai](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/ai.md) | `src/server/game/AI/`, `CoreAI/`, `ScriptedAI/`, `SmartScripts/` |
| [anticheat](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/anticheat.md) | `src/server/game/Entities/Player/Player.cpp`, `src/server/game/Handlers/MovementHandler.cpp`, `src/server/game/Server/WorldSession.cpp` |
| [auctionhouse](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/auctionhouse.md) | `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/`, `src/server/game/Handlers/AuctionHouseHandler.cpp`, `src/server/game/Server/Packets/AuctionHousePackets.{h,cpp}` |
| [auctionhousebot](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/auctionhousebot.md) | `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/` |
| [battlefield](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/battlefield.md) | `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/`, `/home/server/woltk-trinity-legacy/src/server/scripts/Battlefield/` |
| [battlegrounds](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/battlegrounds.md) | `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/` |
| [battlepets](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/battlepets.md) | `src/server/game/BattlePets/`, `src/server/game/Handlers/BattlePetHandler.cpp`, `src/server/game/Server/Packets/BattlePetPackets.{h,cpp}` |
| [blackmarket](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/blackmarket.md) | `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/`, `src/server/game/Handlers/BlackMarketHandler.cpp`, `src/server/game/Server/Packets/BlackMarketPackets.{h,cpp}` |
| [bnetserver](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/bnetserver.md) | `src/server/bnetserver/` |
| [cache](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/cache.md) | `/home/server/woltk-trinity-legacy/src/server/game/Cache/` |
| [calendar](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/calendar.md) | `/home/server/woltk-trinity-legacy/src/server/game/Calendar/`, `src/server/game/Handlers/CalendarHandler.cpp` |
| [chat](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/chat.md) | `src/server/game/Chat/`, `src/server/game/Handlers/ChatHandler.cpp`, `src/server/game/Handlers/ChannelHandler.cpp` |
| [client tools](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/client-tools.md) | `/home/server/woltk-trinity-legacy/src/tools/` |
| [combat dealdamage](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/combat-dealdamage.md) | `src/server/game/Entities/Unit/Unit.cpp`, `src/server/game/Entities/Unit/Unit.h` |
| [combat manager](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/combat-manager.md) | `src/server/game/Combat/CombatManager.{h,cpp}`, `src/server/game/Server/Packets/CombatPackets.cpp` |
| [combat threat](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/combat-threat.md) | `src/server/game/Combat/ThreatManager.{h,cpp}`, `src/server/game/Combat/ThreatReference` |
| [combat](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/combat.md) | `src/server/game/Combat/`, `src/server/game/Server/Packets/CombatPackets.{h,cpp}`, `Entities/Unit/Unit.cpp` |
| [commands](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/commands.md) | `src/server/scripts/Commands/`, `src/server/game/Chat/ChatCommands/`, `src/server/worldserver/CommandLine/`, `src/server/worldserver/RemoteAccess/` |
| [common collision](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/common-collision.md) | `src/common/Collision/` |
| [common](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/common.md) | `/home/server/woltk-trinity-legacy/src/common/`, `Collision/` |
| [conditions](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/conditions.md) | `src/server/game/Conditions/` |
| [crypto](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/crypto.md) | `/home/server/woltk-trinity-legacy/src/common/Cryptography/` |
| [database framework](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/database-framework.md) | `src/server/database/Database/`, `src/server/database/Updater/` |
| [datastores](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/datastores.md) | `/home/server/woltk-trinity-legacy/src/server/game/DataStores/` |
| [db schemas](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/db-schemas.md) | `/home/server/woltk-trinity-legacy/sql/` |
| [dungeonfinding](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/dungeonfinding.md) | `src/server/game/DungeonFinding/`, `src/server/game/Handlers/LFGHandler.cpp` |
| [entities areatrigger](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-areatrigger.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/` |
| [entities conversation](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-conversation.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/` |
| [entities corpse](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-corpse.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/` |
| [entities creature](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-creature.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/` |
| [entities dynamicobject](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-dynamicobject.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/` |
| [entities gameobject](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-gameobject.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/` |
| [entities object](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-object.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/` |
| [entities player](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-player.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/` |
| [entities sceneobject](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-sceneobject.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/` |
| [entities totem](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-totem.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/` |
| [entities transport](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-transport.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/`, `src/server/game/Maps/MapObject.h` |
| [entities unit](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-unit.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/` |
| [entities vehicle](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities-vehicle.md) | `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/` |
| [entities](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/entities.md) | `src/server/game/Entities/` |
| [events](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/events.md) | `src/server/game/Events/`, `src/server/scripts/Events/` |
| [globals](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/globals.md) | `src/server/game/Globals/` |
| [grids](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/grids.md) | `src/server/game/Grids/` |
| [groups](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/groups.md) | `src/server/game/Groups/`, `src/server/game/Handlers/GroupHandler.cpp` |
| [guilds](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/guilds.md) | `src/server/game/Guilds/`, `src/server/game/Handlers/GuildHandler.cpp`, `src/server/game/GuildFinder/GuildFinderMgr.{h,cpp}` |
| [handlers](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/handlers.md) | `/home/server/woltk-trinity-legacy/src/server/game/Handlers/` |
| [instances](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/instances.md) | `/home/server/woltk-trinity-legacy/src/server/game/Instances/` |
| [inventory](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/inventory.md) | `src/server/game/Entities/Item/`, `src/server/game/Entities/Player/PlayerStorage.cpp`, `Player.cpp`, `src/server/game/Handlers/ItemHandler.cpp` |
| [localization](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/localization.md) | `src/common/Common.h`, `src/common/Utilities/Locales.{h,cpp}`, `src/server/game/Globals/ObjectMgr.{h,cpp}`, `src/server/game/Texts/CreatureTextMgr.{h,cpp}` |
| [logging](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/logging.md) | `/home/server/woltk-trinity-legacy/src/common/Logging/`, `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.{h,cpp}` |
| [loot](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/loot.md) | `src/server/game/Loot/`, `src/server/game/Handlers/LootHandler.cpp` |
| [mails](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/mails.md) | `/home/server/woltk-trinity-legacy/src/server/game/Mails/`, `src/server/game/Handlers/MailHandler.cpp` |
| [maps](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/maps.md) | `src/server/game/Maps/` |
| [migration perf strategy](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/migration-perf-strategy.md) | `/home/server/rustycore/docs/migration/` |
| [migration test strategy](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/migration-test-strategy.md) | `/home/server/rustycore/docs/migration/` |
| [miscellaneous](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/miscellaneous.md) | `src/server/game/Miscellaneous/` |
| [movement generators](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/movement-generators.md) | `src/server/game/Movement/MotionMaster.{h,cpp}`, `src/server/game/Movement/MovementGenerator.{h,cpp}`, `src/server/game/Movement/MovementGenerators/*.{h,cpp}`, `src/server/game/Movement/AbstractFollower.{h,cpp}` |
| [movement pathgen](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/movement-pathgen.md) | `src/server/game/Movement/PathGenerator.{h,cpp}`, `src/common/Collision/Management/MMapManager.{h,cpp}`, `src/common/Collision/Maps/MMapDefines.h`, `dep/recastnavigation/Detour/` |
| [movement spline](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/movement-spline.md) | `src/server/game/Movement/Spline/MoveSpline.{h,cpp}`, `SplineImpl.h`, `MoveSplineFlag.h`, `MoveSplineInitArgs.h` |
| [movement](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/movement.md) | `src/server/game/Movement/`, `src/server/game/Server/Packets/MovementPackets.{h,cpp}` |
| [opcodes inventory](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/opcodes-inventory.md) | `src/server/game/Server/Protocol/Opcodes.{h,cpp}` |
| [outdoorpvp](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/outdoorpvp.md) | `src/server/game/OutdoorPvP/`, `src/server/scripts/OutdoorPvP/` |
| [packets by domain](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/packets-by-domain.md) | `/home/server/woltk-trinity-legacy/src/server/game/Server/Packets/` |
| [petitions](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/petitions.md) | `src/server/game/Petitions/`, `src/server/game/Handlers/PetitionsHandler.cpp` |
| [pets](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/pets.md) | `src/server/game/Entities/Pet/`, `src/server/game/Handlers/PetHandler.cpp`, `src/server/game/Server/Packets/PetPackets.{h,cpp}`, `src/server/game/Handlers/NPCHandler.cpp` |
| [phasing](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/phasing.md) | `src/server/game/Phasing/` |
| [pools](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/pools.md) | `src/server/game/Pools/` |
| [proto](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/proto.md) | `src/server/proto/` |
| [quests](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/quests.md) | `src/server/game/Quests/`, `src/server/game/Pools/QuestPools.*`, `src/server/game/Handlers/QuestHandler.cpp` |
| [reputation](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/reputation.md) | `/home/server/woltk-trinity-legacy/src/server/game/Reputation/`, `src/server/game/Server/Packets/ReputationPackets.cpp/.h` |
| [scenarios](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/scenarios.md) | `src/server/game/Scenarios/` |
| [scripting](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/scripting.md) | `src/server/game/Scripting/` |
| [scripts icc](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/scripts-icc.md) | `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/` |
| [scripts ulduar](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/scripts-ulduar.md) | `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/` |
| [scripts](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/scripts.md) | `src/server/scripts/` |
| [server](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/server.md) | `src/server/game/Server/` |
| [services](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/services.md) | `src/server/game/Services/` |
| [shared datastores](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/shared-datastores.md) | `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/`, `/home/server/woltk-trinity-legacy/src/common/DataStores/` |
| [shared dynamic](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/shared-dynamic.md) | `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/` |
| [shared json](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/shared-json.md) | `/home/server/woltk-trinity-legacy/src/server/shared/JSON/` |
| [shared networking](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/shared-networking.md) | `/home/server/woltk-trinity-legacy/src/server/shared/Networking/` |
| [shared packets](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/shared-packets.md) | `/home/server/woltk-trinity-legacy/src/server/shared/Packets/` |
| [shared realm](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/shared-realm.md) | `/home/server/woltk-trinity-legacy/src/server/shared/Realm/` |
| [shared secrets](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/shared-secrets.md) | `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/` |
| [skills](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/skills.md) | `/home/server/woltk-trinity-legacy/src/server/game/Skills/`, `src/server/game/Handlers/SkillHandler.cpp` |
| [social](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/social.md) | `src/server/game/Handlers/SocialHandler.cpp`, `src/server/game/Entities/Player/SocialMgr.{h,cpp}`, `src/server/game/Globals/ObjectMgr.cpp` |
| [spells aura](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/spells-aura.md) | `src/server/game/Spells/Auras/` |
| [spells cast](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/spells-cast.md) | `src/server/game/Spells/Spell.{h,cpp}`, `SpellCastRequest.h` |
| [spells effects](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/spells-effects.md) | `src/server/game/Spells/SpellEffects.cpp` |
| [spells info](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/spells-info.md) | `src/server/game/Spells/SpellInfo.{h,cpp}` |
| [spells mgr](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/spells-mgr.md) | `src/server/game/Spells/SpellMgr.{h,cpp}` |
| [spells](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/spells.md) | `src/server/game/Spells/`, `Auras/` |
| [storages](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/storages.md) | `/home/server/woltk-trinity-legacy/src/server/game/Storages/` |
| [support](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/support.md) | `/home/server/woltk-trinity-legacy/src/server/game/Support/` |
| [texts](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/texts.md) | `/home/server/woltk-trinity-legacy/src/server/game/Texts/` |
| [time](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/time.md) | `/home/server/woltk-trinity-legacy/src/server/game/Time/` |
| [tools](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/tools.md) | `/home/server/woltk-trinity-legacy/src/server/game/Tools/` |
| [warden](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/warden.md) | `src/server/game/Warden/`, `enuminfo_WardenCheckMgr.cpp`, `Modules/WardenModuleWin.h`, `Modules/WardenModuleMac.h` |
| [weather](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/weather.md) | `/home/server/woltk-trinity-legacy/src/server/game/Weather/` |
| [world](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/world.md) | `src/server/game/World/` |
| [worldserver](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/worldserver.md) | `src/server/worldserver/` |

## Cross-system audits

- [Creature runtime scope and original C++ source matrix](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/creature-port-no-gaps-plan.md)
- [R8 evidence categories and historical coverage counts](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/honest-progress-audit.md)
- [Original login packet ordering and AddPlayerToMap plan](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/login-world-load-cpp-parity-plan.md)
- [Login-to-world packet comparisons and action-specific findings](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/world-load-audit.md)
