-- Recoverable battle-pet trainer purchase saga (issue #161). The legacy C++
-- server charges money in memory at `Trainer::TeachSpell` and only persists
-- both sides at the next `Player::SaveToDB`, committing Character DB first
-- and Login DB second (`Player.cpp:19336-19344`); a crash between the two
-- commits keeps the charge and loses the pet, and `BattlePetMgr::SaveToDB`
-- clears `SaveInfo` when statements are appended (`BattlePetMgr.cpp:377`),
-- so the loss is silent. There is no portable atomic transaction across the
-- two database pools, so Rust records one durable command per purchase
-- attempt: the guarded money deduction and this row commit in one Character
-- DB transaction, the pet is then applied once through the account battle-pet
-- owner (issue #160) using `request_key` as the `battle_pet_add_requests`
-- receipt identity, and a terminal apply failure is compensated exactly once.
-- status: 0 PendingApplication, 1 Completed, 2 CompensationPending,
-- 3 Compensated, 4 TerminalFailure.
CREATE TABLE IF NOT EXISTS `character_battle_pet_purchase` (
  `request_key` binary(16) NOT NULL,
  `guid` bigint unsigned NOT NULL,
  `account_id` int unsigned NOT NULL,
  `trainer_id` int unsigned NOT NULL,
  `spell_id` int unsigned NOT NULL,
  `species` int unsigned NOT NULL,
  `breed` smallint unsigned NOT NULL,
  `quality` tinyint unsigned NOT NULL,
  `display_id` int unsigned NOT NULL,
  `level` smallint unsigned NOT NULL,
  `price` int unsigned NOT NULL,
  `money_before` bigint unsigned NOT NULL,
  `money_after` bigint unsigned NOT NULL,
  `status` tinyint unsigned NOT NULL,
  `failure_reason` varchar(64) DEFAULT NULL,
  `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`request_key`),
  KEY `idx_guid_status` (`guid`,`status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
