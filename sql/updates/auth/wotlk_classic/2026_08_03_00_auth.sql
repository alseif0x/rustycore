-- Caged item ObjectGuids are globally unique and never reused. The durable
-- uncage receipt must therefore remain unique even if the item is traded to a
-- different Battle.net account after pet creation but before item cleanup.
--
-- This is intentionally an ALTER in a new update rather than a change to the
-- preceding CREATE TABLE IF NOT EXISTS: databases that already applied the
-- original issue migration must receive the stronger invariant as well.
-- The previous composite key admitted one bad historical grant per account
-- for the same globally unique source item. Preserve the earliest allocated
-- pet receipt and discard only later duplicate receipts before tightening the
-- key; the pets themselves remain intact for explicit operator remediation.
DELETE duplicateReceipt
FROM `battle_pet_add_requests` duplicateReceipt
INNER JOIN `battle_pet_add_requests` canonicalReceipt
  ON canonicalReceipt.`requestKey` = duplicateReceipt.`requestKey`
 AND canonicalReceipt.`battlePetGuid` < duplicateReceipt.`battlePetGuid`;

ALTER TABLE `battle_pet_add_requests`
  MODIFY COLUMN `battlenetAccountId` int unsigned NOT NULL,
  DROP PRIMARY KEY,
  ADD PRIMARY KEY (`requestKey`),
  ADD KEY `idx_battle_pet_add_requests_account` (`battlenetAccountId`);

-- A monotonically increasing epoch complements the connection-scoped named
-- lock. Every durable journal mutation locks and validates this row inside
-- its transaction, so work queued by a former world process cannot commit
-- after a replacement owner has taken over the account.
CREATE TABLE IF NOT EXISTS `battle_pet_account_fences` (
  `battlenetAccountId` int unsigned NOT NULL,
  `generation` bigint unsigned NOT NULL DEFAULT 0,
  `operationSerial` bigint unsigned NOT NULL DEFAULT 0,
  PRIMARY KEY (`battlenetAccountId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
