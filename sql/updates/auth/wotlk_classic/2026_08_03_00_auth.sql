-- Caged item ObjectGuids are globally unique and never reused. The durable
-- uncage receipt must therefore remain unique even if the item is traded to a
-- different Battle.net account after pet creation but before item cleanup.
--
-- This is intentionally an ALTER in a new update rather than a change to the
-- preceding CREATE TABLE IF NOT EXISTS: databases that already applied the
-- original issue migration must receive the stronger invariant as well.
ALTER TABLE `battle_pet_add_requests`
  MODIFY COLUMN `battlenetAccountId` int unsigned NOT NULL,
  DROP PRIMARY KEY,
  ADD PRIMARY KEY (`requestKey`),
  ADD KEY `idx_battle_pet_add_requests_account` (`battlenetAccountId`);
