-- A receipt is visible if and only if its entire shared money payout committed.
CREATE TABLE `loot_money_claim_receipt` (
  `claim_id` binary(16) NOT NULL,
  `payouts` json NOT NULL,
  PRIMARY KEY (`claim_id`)
) ENGINE=InnoDB;

-- Durable identity and current restock deadline for each persistent chest
-- source. The generation advances only after the stored deadline has elapsed.
CREATE TABLE `loot_money_source_head` (
  `realm_id` smallint unsigned NOT NULL,
  `map_id` int unsigned NOT NULL,
  `instance_id` int unsigned NOT NULL,
  `spawn_id` bigint unsigned NOT NULL,
  `generation` bigint unsigned NOT NULL DEFAULT 0,
  `restock_deadline` bigint DEFAULT NULL,
  PRIMARY KEY (`realm_id`, `map_id`, `instance_id`, `spawn_id`)
) ENGINE=InnoDB;

-- One opaque claim id is immutably bound to each source generation and loot
-- scope. Shared scope uses owner 0; personal scope uses the character counter.
-- The spent bit is updated in the same Character transaction as payout rows.
CREATE TABLE `loot_money_source_binding` (
  `realm_id` smallint unsigned NOT NULL,
  `map_id` int unsigned NOT NULL,
  `instance_id` int unsigned NOT NULL,
  `spawn_id` bigint unsigned NOT NULL,
  `generation` bigint unsigned NOT NULL,
  `scope_type` tinyint unsigned NOT NULL,
  `scope_owner` bigint unsigned NOT NULL,
  `claim_id` binary(16) NOT NULL,
  `spent` tinyint unsigned NOT NULL DEFAULT 0,
  PRIMARY KEY (`realm_id`, `map_id`, `instance_id`, `spawn_id`, `generation`, `scope_type`, `scope_owner`),
  UNIQUE KEY `uq_loot_money_source_claim_id` (`claim_id`),
  CONSTRAINT `fk_loot_money_source_binding_head`
    FOREIGN KEY (`realm_id`, `map_id`, `instance_id`, `spawn_id`)
    REFERENCES `loot_money_source_head` (`realm_id`, `map_id`, `instance_id`, `spawn_id`)
) ENGINE=InnoDB;
