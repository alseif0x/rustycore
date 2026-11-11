-- RustyCore's explicit two-database creation acknowledgement contract.
-- Apply only to the selected Forever Characters database, before enabling its
-- creation writer. This is a server extension, not a target C++ table.
-- Pending rows are recovery evidence; never delete them as a startup shortcut.
CREATE TABLE `rustycore_forever_creation_journal` (
  `player_guid` BIGINT UNSIGNED NOT NULL,
  `account_id` INT UNSIGNED NOT NULL,
  `realm_address` INT UNSIGNED NOT NULL,
  `save_time` INT UNSIGNED NOT NULL,
  `format_version` TINYINT UNSIGNED NOT NULL,
  PRIMARY KEY (`player_guid`)
) ENGINE=InnoDB;
