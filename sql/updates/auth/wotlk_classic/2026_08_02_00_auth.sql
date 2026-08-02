-- Global allocator shared by every realm using this Login DB. Allocations
-- lock this singleton row only for one short transaction; world-server
-- processes must not hold a Login DB-wide lock for their complete lifetime.
CREATE TABLE IF NOT EXISTS `battle_pet_guid_sequence` (
  `singleton` tinyint unsigned NOT NULL,
  `nextGuid` bigint unsigned NOT NULL,
  PRIMARY KEY (`singleton`),
  CONSTRAINT `chk_battle_pet_guid_sequence_singleton` CHECK (`singleton` = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO `battle_pet_guid_sequence` (`singleton`, `nextGuid`)
SELECT 1, COALESCE(MAX(`guid`), 0) + 1 FROM `battle_pets`
ON DUPLICATE KEY UPDATE `nextGuid` = GREATEST(`nextGuid`, VALUES(`nextGuid`));

-- Durable idempotency receipts for account-atomic battle-pet creation.
-- The receipt and battle_pets row are inserted in one Login DB transaction.
CREATE TABLE IF NOT EXISTS `battle_pet_add_requests` (
  `battlenetAccountId` int NOT NULL,
  `requestKey` binary(16) NOT NULL,
  `battlePetGuid` bigint NOT NULL,
  `species` int NOT NULL,
  `breed` smallint NOT NULL,
  `displayId` int NOT NULL,
  `level` smallint NOT NULL,
  `exp` smallint NOT NULL DEFAULT '0',
  `health` int NOT NULL DEFAULT '0',
  `quality` tinyint NOT NULL,
  `flags` smallint NOT NULL DEFAULT '0',
  `name` varchar(12) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '',
  `nameTimestamp` bigint NOT NULL DEFAULT '0',
  `owner` bigint DEFAULT NULL,
  PRIMARY KEY (`battlenetAccountId`, `requestKey`),
  UNIQUE KEY `uq_battle_pet_add_requests_guid` (`battlePetGuid`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- requestKey is the consumed caged item's globally unique ObjectGuid. Rows are
-- intentionally retained after pet deletion: the source item must never grant
-- a second pet, and durable item GUIDs are not reused.
