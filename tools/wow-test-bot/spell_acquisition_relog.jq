def persistence_matches:
  . as $e |
  .persistence as $p |
  if .persistence_verified != true or ($p | type) != "object" then false
  elif $p == {kind: "direct_spell"} then .saved_spell == true and .observed_skill_root == null
  elif ($p | keys) == ["id", "kind", "max", "value"] and $p.kind == "skill" then
    ($p.id | type == "number" and . > 0 and . <= 65535 and . == floor) and
    ($p.value | type == "number" and . > 0 and . <= 65535 and . == floor) and
    ($p.max | type == "number" and . >= $p.value and . <= 65535 and . == floor) and
    $e.saved_spell == false and
    $e.observed_skill_root == {id: $p.id, value: $p.value, max: $p.max}
  else false end;

.[0].results[0].spell_acquisition as $first |
.[1].results[0].spell_acquisition as $second |
if length != 2 or ($first | type) != "object" or ($second | type) != "object" then
  error("missing action-specific acquisition evidence")
elif $first.learned_on_instance != true or $first.verified_at_login != false or
     $second.verified_at_login != true or
     ($first | persistence_matches | not) or ($second | persistence_matches | not) then
  error("acquisition or relogin was not verified")
elif $first.expected_spell != $second.expected_spell or $first.persistence != $second.persistence or
     $first.saved_money != $first.expected_saved_money or
     $first.saved_money != $second.money_before or $first.saved_money != $second.saved_money or
     $second.saved_money != $second.expected_saved_money then
  error("acquisition identity or persisted money changed")
else {spell_acquisition_relog_verified: true, first: $first, second: $second} end
