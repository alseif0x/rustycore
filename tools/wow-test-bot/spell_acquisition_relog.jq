.[0].results[0].spell_acquisition as $first |
.[1].results[0].spell_acquisition as $second |
if length != 2 or ($first | type) != "object" or ($second | type) != "object" then
  error("missing action-specific acquisition evidence")
elif $first.learned_on_instance != true or $first.verified_at_login != false or
     $second.verified_at_login != true or $first.saved_spell != true or $second.saved_spell != true then
  error("acquisition or relogin was not verified")
elif $first.expected_spell != $second.expected_spell or
     $first.saved_money != $first.expected_saved_money or
     $first.saved_money != $second.money_before or $first.saved_money != $second.saved_money then
  error("acquisition identity or persisted money changed")
else {spell_acquisition_relog_verified: true, first: $first, second: $second} end
