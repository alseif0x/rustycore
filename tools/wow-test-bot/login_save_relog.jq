def valid:
  .login_only == true and (.results | length == 1) and
  all(.results[];
    .account == "TESTBOT1@bot.local" and
    .world_auth == true and .enum_characters == true and
    .player_login_verified == true and .login_stream_drained == true and
    .login_save.logout_confirmed == true and .login_save.offline == true and
    .login_save.retained_existing_rows == true and
    .login_save.logout_time_after > .login_save.logout_time_before and
    (.login_save.after | type == "object" and length == 6) and
    (.login_save.known_spells | type == "array") and
    (.login_save.favorite_spells | type == "array"));
if length != 2 or (all(.[]; valid) | not) then
  error("missing verified login/save evidence")
elif .[0].results[0].character_guid != .[1].results[0].character_guid or
     .[0].results[0].account_id != .[1].results[0].account_id then
  error("relogin identity changed")
elif .[0].results[0].login_save.logout_time_after != .[1].results[0].login_save.logout_time_before then
  error("save marker changed between sessions")
elif .[0].results[0].login_save.after != .[1].results[0].login_save.before or
     .[0].results[0].login_save.after != .[1].results[0].login_save.after then
  error("saved spell/skill/equipment/reputation projections changed across relog")
elif .[0].results[0].login_save.known_spells != .[1].results[0].login_save.known_spells or
     .[0].results[0].login_save.favorite_spells != .[1].results[0].login_save.favorite_spells then
  error("known/favorite spell packets changed across relog")
else .[1] + {login_save_relog_verified: true} end
