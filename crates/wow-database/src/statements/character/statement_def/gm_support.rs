//! Character GM support SQL values.
pub(super) const INS_BUG_REPORT: &str = "INSERT INTO bugreport (type, content) VALUES(?, ?)";
pub(super) const SEL_GM_BUGS: &str = {
    "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment FROM gm_bug"
};
pub(super) const REP_GM_BUG: &str = {
    "REPLACE INTO gm_bug (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_GM_BUG: &str = "DELETE FROM gm_bug WHERE id = ?";
pub(super) const DEL_ALL_GM_BUGS: &str = "DELETE FROM gm_bug";
pub(super) const SEL_GM_COMPLAINTS: &str = {
    "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, targetCharacterGuid, reportType, reportMajorCategory, reportMinorCategoryFlags, reportLineIndex, assignedTo, closedBy, comment FROM gm_complaint"
};
pub(super) const REP_GM_COMPLAINT: &str = {
    "REPLACE INTO gm_complaint (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, targetCharacterGuid, reportType, reportMajorCategory, reportMinorCategoryFlags, reportLineIndex, assignedTo, closedBy, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_GM_COMPLAINT: &str = "DELETE FROM gm_complaint WHERE id = ?";
pub(super) const SEL_GM_COMPLAINT_CHATLINES: &str = {
    "SELECT timestamp, text FROM gm_complaint_chatlog WHERE complaintId = ? ORDER BY lineId ASC"
};
pub(super) const INS_GM_COMPLAINT_CHATLINE: &str = {
    "INSERT INTO gm_complaint_chatlog (complaintId, lineId, timestamp, text) VALUES (?, ?, ?, ?)"
};
pub(super) const DEL_GM_COMPLAINT_CHATLOG: &str =
    { "DELETE FROM gm_complaint_chatlog WHERE complaintId = ?" };
pub(super) const DEL_ALL_GM_COMPLAINTS: &str = "DELETE FROM gm_complaint";
pub(super) const DEL_ALL_GM_COMPLAINT_CHATLOGS: &str = "DELETE FROM gm_complaint_chatlog";
pub(super) const SEL_GM_SUGGESTIONS: &str = {
    "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment FROM gm_suggestion"
};
pub(super) const REP_GM_SUGGESTION: &str = {
    "REPLACE INTO gm_suggestion (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ? ,? ,?)"
};
pub(super) const DEL_GM_SUGGESTION: &str = "DELETE FROM gm_suggestion WHERE id = ?";
pub(super) const DEL_ALL_GM_SUGGESTIONS: &str = "DELETE FROM gm_suggestion";
