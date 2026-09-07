#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
fixture=$(jq -n '
  {logout_confirmed:false,disconnect_confirmed:true,login_account_offline:true,offline:true,retained_existing_rows:true,
   logout_time_before:1,logout_time_after:2,before:{a:1,b:2,c:3,d:4,e:5,f:6},
   after:{a:1,b:2,c:3,d:4,e:5,f:6},known_spells:[10],favorite_spells:[]} as $save |
  {login_only:true,results:[{account:"TESTBOT1@bot.local",account_id:1,character_guid:2,
   world_auth:true,enum_characters:true,player_login_verified:true,login_stream_drained:true,
   login_save:$save}]} | [.,.] |
   .[1].results[0].login_save.logout_time_before=2 |
   .[1].results[0].login_save.logout_time_after=3 |
   .[1].results[0].login_save.logout_confirmed=true |
   .[1].results[0].login_save.disconnect_confirmed=false')
jq -e -f login_disconnect_relog.jq <<<"$fixture" >/dev/null
for mutation in \
  '.[0].results[0].login_save.disconnect_confirmed=false' \
  '.[0].results[0].login_save.logout_confirmed=true' \
  '.[1].results[0].login_save.disconnect_confirmed=true' \
  '.[1].results[0].login_save.logout_confirmed=false' \
  '.[0].results[0].login_save.offline=false' \
  '.[0].results[0].login_save.login_account_offline=false' \
  '.[1].results[0].login_save.logout_time_before=1' \
  '.[0].results[0].login_save.logout_time_after=1' \
  '.[1].results[0].character_guid=3' \
  '.[1].results[0].login_save.before.a=99' \
  '.[1].results[0].login_save.after.a=99' \
  '.[1].results[0].login_save.known_spells=[20]'; do
  changed=$(jq "$mutation" <<<"$fixture")
  if jq -e -f login_disconnect_relog.jq <<<"$changed" >/dev/null 2>&1; then
    echo "FAIL: accepted $mutation" >&2
    exit 1
  fi
done
echo 'login-disconnect-relog: positive + 12 negative evidence checks passed'
