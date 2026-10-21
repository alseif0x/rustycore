-- example.milestones: per-character milestone counts (#1444 H6).
-- The module's own table, named with its prefix mod_example_milestones_.
-- Nothing writes it yet: the module API gives modules no database access;
-- durable module state is #583. This file shows the SQL lifecycle a module
-- uses: never edit it once applied, add a new migration instead.
CREATE TABLE IF NOT EXISTS `mod_example_milestones_counts` (
  `guid` BIGINT UNSIGNED NOT NULL COMMENT 'characters.guid',
  `kills` INT UNSIGNED NOT NULL DEFAULT 0,
  `quests_rewarded` INT UNSIGNED NOT NULL DEFAULT 0,
  `best_level` TINYINT UNSIGNED NOT NULL DEFAULT 0,
  PRIMARY KEY (`guid`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
