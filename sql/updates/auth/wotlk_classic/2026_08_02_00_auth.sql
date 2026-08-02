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
