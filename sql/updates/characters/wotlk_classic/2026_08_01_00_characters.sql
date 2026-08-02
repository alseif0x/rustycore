-- Attributes an ambiguous trainer spell-acquisition COMMIT to one exact
-- attempt. One latest token per character is sufficient: a later operation
-- replaces it and therefore cannot be mistaken for the earlier attempt.
CREATE TABLE IF NOT EXISTS `character_spell_acquisition_operation` (
  `guid` bigint unsigned NOT NULL,
  `operation_token` binary(16) NOT NULL,
  PRIMARY KEY (`guid`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
