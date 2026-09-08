if (.action == "trainer" or .action == "cast") and
  (.expected_spell | type == "number" and . > 0 and . == floor)
then {action: "verify", expected_spell: .expected_spell,
  persistence: (if has("persistence") then .persistence else {kind: "direct_spell"} end)}
else error("requires a typed trainer or cast acquisition plan") end
