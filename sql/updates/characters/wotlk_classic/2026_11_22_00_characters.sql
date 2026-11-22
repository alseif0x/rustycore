-- A receipt is visible if and only if its entire shared money payout committed.
CREATE TABLE `loot_money_claim_receipt` (
  `claim_id` binary(16) NOT NULL,
  `payouts` json NOT NULL,
  PRIMARY KEY (`claim_id`)
) ENGINE=InnoDB;
