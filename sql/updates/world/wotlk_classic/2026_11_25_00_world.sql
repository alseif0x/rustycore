START TRANSACTION;

-- The Stormwind refugee event was captured for retail build 50585 (10.x
-- 2023_07_31_00_world.sql). Its phase 11595 is absent in Wrath 54261.
-- Keep the imported rows and paths. Map::InitSpawnGroupState evaluates
-- conditions even for manual groups. Exclude these map-0 spawns explicitly.
INSERT INTO spawn_group_template (groupId, groupName, groupFlags)
SELECT 34311595, 'Wrath - Inactive retail Stormwind refugee event', 4
WHERE NOT EXISTS (
    SELECT 1 FROM spawn_group_template WHERE groupId = 34311595
      AND groupName = 'Wrath - Inactive retail Stormwind refugee event' AND groupFlags = 4
);

INSERT INTO conditions
    (SourceTypeOrReferenceId, SourceEntry, ConditionTypeOrReference, ConditionValue1,
     NegativeCondition, Comment)
SELECT 33, 34311595, 22, 0, 1, 'Wrath: exclude retail refugees from Eastern Kingdoms'
WHERE NOT EXISTS (
    SELECT 1 FROM conditions WHERE SourceTypeOrReferenceId = 33 AND SourceGroup = 0
      AND SourceEntry = 34311595 AND SourceId = 0 AND ElseGroup = 0
      AND ConditionTypeOrReference = 22 AND ConditionTarget = 0
      AND ConditionValue1 = 0 AND ConditionValue2 = 0 AND ConditionValue3 = 0
      AND ConditionStringValue1 = '' AND NegativeCondition = 1
);

INSERT INTO spawn_group (groupId, spawnType, spawnId)
SELECT 34311595, 0, c.guid
FROM creature c
WHERE c.map = 0 AND c.zoneId = 1519 AND c.PhaseId = 11595 AND c.VerifiedBuild = 50585
  AND (c.guid, c.id) IN (
      (851266, 3681), (851267, 3681), (851269, 4423), (851274, 14555),
      (851275, 14556), (851276, 4730), (851279, 14602), (851281, 4753),
      (851283, 12358), (851284, 12359), (851285, 12360), (851286, 35168),
      (851316, 2041), (851318, 3681), (851324, 7553), (851325, 7555),
      (851327, 8665), (851329, 3681), (851330, 3681), (851332, 3681),
      (851333, 3681), (851334, 3681)
  )
  AND NOT EXISTS (
      SELECT 1 FROM spawn_group g WHERE g.spawnType = 0 AND g.spawnId = c.guid
  );

-- Each old position matches a 4.3.4 record. At the same XY the pinned Wrath
-- reference has one matching creature, with its feet within 0.22 units of a
-- floor in the 54261 terrain or collision data. Keep local placement edits.
UPDATE creature c
JOIN (
    SELECT 9432 AS guid, 4963 AS entry, 0 AS map_id, -3822.8799 AS x, -827.8960 AS y, 0.85608 AS old_z, 10.2984 AS wrath_z  -- Mikhail
    UNION ALL SELECT 10873, 4962, 0, -3808.8201, -825.6090, 0.85958, 10.3019  -- Tapoke "Slim" Jahn
    UNION ALL SELECT 260752, 8881, 0, -3796.5300, -827.7170, -0.20033, 9.8649  -- Riding Ram
    UNION ALL SELECT 260753, 8881, 0, -3796.5400, -830.0030, -0.08870, 9.73768  -- Riding Ram
    UNION ALL SELECT 260755, 1457, 0, -3807.1899, -823.6090, 7.71395, 17.1563  -- Samor Festivus
    UNION ALL SELECT 260758, 1456, 0, -3818.6499, -818.8140, 0.10843, 9.55073  -- Kersok Prond
    UNION ALL SELECT 260759, 1217, 0, -3818.0500, -827.2970, 0.08378, 9.67523  -- Glorin Steelbrow
    UNION ALL SELECT 260771, 2911, 0, -3813.6101, -837.8870, 7.72178, 17.1641  -- Archaeologist Flagongut
    UNION ALL SELECT 271109, 11720, 1, 2393.5500, -2525.3301, 108.31800, 111.374  -- Loruk Foreststrider
    UNION ALL SELECT 275083, 842, 0, -10584.7998, 1144.2200, 37.77050, 40.2974  -- Lumberjack
    UNION ALL SELECT 289899, 8917, 0, -7477.8701, -916.0170, 107.38900, 173.522  -- Quarry Slave
    UNION ALL SELECT 289900, 8917, 0, -7471.4199, -927.5990, 131.99500, 174.644  -- Quarry Slave
    UNION ALL SELECT 289907, 8917, 0, -7466.7402, -1002.3200, 117.28900, 172.907  -- Quarry Slave
    UNION ALL SELECT 289908, 8917, 0, -7468.0601, -980.5120, 155.88300, 173.567  -- Quarry Slave
    UNION ALL SELECT 305888, 946, 0, -6606.9502, 388.9250, 394.99301, 390.202  -- Frostmane Novice
    UNION ALL SELECT 320399, 16284, 0, 2309.5100, -5305.8701, 86.79020, 82.0796  -- Argent Medic
    UNION ALL SELECT 324992, 1688, 0, 2084.5701, 1950.0601, 104.61500, 98.4014  -- Night Web Matriarch
    UNION ALL SELECT 325850, 1548, 0, 1722.1400, 720.1630, 48.04240, 53.9895  -- Cursed Darkhound
) p ON p.guid = c.guid AND p.entry = c.id AND p.map_id = c.map
SET c.position_z = p.wrath_z
WHERE ABS(c.position_x - p.x) < 0.01 AND ABS(c.position_y - p.y) < 0.01
  AND ABS(c.position_z - p.old_z) < 0.001
  AND c.PhaseId = 0 AND c.PhaseGroup = 0 AND c.terrainSwapMap = -1;

COMMIT;
