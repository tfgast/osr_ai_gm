#!/bin/bash
# Playtest Pass 2D: Stress Test & Edge Cases
# Run against a live osr-gm-server instance
#
# Known bugs found during testing:
#   oag-7sa5i: RollInitiative clears spell declarations (Declare→Init→Cast fails)
#   oag-t8467: No spell slot tracking (DeclareSpell always succeeds)
set -euo pipefail

PORT="${OSR_GM_PORT:-9879}"
TOKEN="${OSR_GM_TOKEN:-}"

if [ -z "$TOKEN" ]; then
  echo "ERROR: Set OSR_GM_TOKEN env var"
  exit 1
fi

PASS=0
FAIL=0
KNOWN=0
BUGS=""

gm() {
  curl -s -X POST "http://127.0.0.1:${PORT}/api/v1/gm" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$1"
}

# Check response for success/failure
check() {
  local label="$1"
  local expect="$2"  # "ok" or "err"
  local resp="$3"

  local success
  success=$(echo "$resp" | python3 -c "import sys,json; print(json.load(sys.stdin).get('success',''))" 2>/dev/null || echo "PARSE_FAIL")

  if [ "$expect" = "ok" ] && [ "$success" = "True" ]; then
    PASS=$((PASS+1))
    echo "  PASS: $label"
  elif [ "$expect" = "err" ] && [ "$success" = "False" ]; then
    PASS=$((PASS+1))
    echo "  PASS: $label (expected error)"
  elif [ "$expect" = "ok" ] && [ "$success" = "False" ]; then
    FAIL=$((FAIL+1))
    local errmsg
    errmsg=$(echo "$resp" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error',''))" 2>/dev/null || echo "unknown")
    echo "  FAIL: $label -> $errmsg"
    BUGS="${BUGS}\nFAIL: $label -> $errmsg"
  elif [ "$expect" = "err" ] && [ "$success" = "True" ]; then
    FAIL=$((FAIL+1))
    echo "  FAIL: $label (expected error but got success)"
    BUGS="${BUGS}\nFAIL: $label (expected error but got success)"
  else
    FAIL=$((FAIL+1))
    echo "  FAIL: $label (parse failure or unexpected: $success)"
    BUGS="${BUGS}\nFAIL: $label (response: $resp)"
  fi
}

# Mark a known bug - test expected to fail
known_bug() {
  local label="$1"
  local bead="$2"
  KNOWN=$((KNOWN+1))
  echo "  KNOWN_BUG ($bead): $label"
}

echo "========================================"
echo "PHASE 1: Party Limits"
echo "========================================"

# Create one of each basic class
R=$(gm '{"id":"p1-1","command":{"type":"CreateCharacter","params":{"name":"Grond","class":"Fighter","alignment":"Neutral","abilities":[16,9,9,9,12,9]}}}')
check "Create Fighter Grond" "ok" "$R"

R=$(gm '{"id":"p1-2","command":{"type":"CreateCharacter","params":{"name":"Brother Cadfael","class":"Cleric","alignment":"Lawful","abilities":[9,9,14,9,9,12]}}}')
check "Create Cleric" "ok" "$R"

R=$(gm '{"id":"p1-3","command":{"type":"CreateCharacter","params":{"name":"Filch","class":"Thief","alignment":"Neutral","abilities":[9,9,9,14,9,9]}}}')
check "Create Thief" "ok" "$R"

R=$(gm '{"id":"p1-4","command":{"type":"CreateCharacter","params":{"name":"Mystara","class":"Magic-User","alignment":"Neutral","abilities":[9,14,9,9,9,9]}}}')
check "Create Magic-User" "ok" "$R"

R=$(gm '{"id":"p1-5","command":{"type":"CreateCharacter","params":{"name":"Elrohir","class":"Elf","alignment":"Neutral","abilities":[12,14,9,9,9,9]}}}')
check "Create Elf" "ok" "$R"

R=$(gm '{"id":"p1-6","command":{"type":"CreateCharacter","params":{"name":"Thorin","class":"Dwarf","alignment":"Lawful","abilities":[12,9,9,9,14,9]}}}')
check "Create Dwarf" "ok" "$R"

R=$(gm '{"id":"p1-7","command":{"type":"CreateCharacter","params":{"name":"Pippin","class":"Halfling","alignment":"Lawful","abilities":[12,9,9,14,9,9]}}}')
check "Create Halfling" "ok" "$R"

# Edge-case abilities: all 3s - Fighter has NO ability requirements per OSE rules
R=$(gm '{"id":"p1-8","command":{"type":"CreateCharacter","params":{"name":"Weakling","class":"Fighter","alignment":"Neutral","abilities":[3,3,3,3,3,3]}}}')
check "Create Fighter with all 3s (no reqs)" "ok" "$R"

# Edge-case abilities: all 18s
R=$(gm '{"id":"p1-9","command":{"type":"CreateCharacter","params":{"name":"Superman","class":"Fighter","alignment":"Neutral","abilities":[18,18,18,18,18,18]}}}')
check "Create with all 18s" "ok" "$R"

# Characters that don't meet requirements: Elf needs INT 9 and STR 9
R=$(gm '{"id":"p1-10","command":{"type":"CreateCharacter","params":{"name":"BadElf","class":"Elf","alignment":"Neutral","abilities":[3,3,3,3,3,3]}}}')
check "Create Elf with all 3s (should fail)" "err" "$R"

# Cleric has NO ability requirements (WIS is prime requisite for XP bonus only)
R=$(gm '{"id":"p1-10b","command":{"type":"CreateCharacter","params":{"name":"Weakcleric","class":"Cleric","alignment":"Lawful","abilities":[9,9,3,9,9,9]}}}')
check "Create Cleric with WIS 3 (no min reqs)" "ok" "$R"

# Empty name rejection (oag-ict3e fix)
R=$(gm '{"id":"p1-11","command":{"type":"CreateCharacter","params":{"name":"","class":"Fighter","alignment":"Neutral","abilities":[16,9,9,9,12,9]}}}')
check "Empty name rejected" "err" "$R"

# Duplicate name rejection (oag-glxrl fix)
R=$(gm '{"id":"p1-12","command":{"type":"CreateCharacter","params":{"name":"Grond","class":"Fighter","alignment":"Neutral","abilities":[16,9,9,9,12,9]}}}')
check "Duplicate name 'Grond' rejected" "err" "$R"

# Very long character name (64 chars)
LONG64=$(python3 -c "print('A'*64)")
R=$(gm "{\"id\":\"p1-13\",\"command\":{\"type\":\"CreateCharacter\",\"params\":{\"name\":\"$LONG64\",\"class\":\"Fighter\",\"alignment\":\"Neutral\",\"abilities\":[16,9,9,9,12,9]}}}")
check "64-char name" "ok" "$R"

# Very long character name (128 chars)
LONG128=$(python3 -c "print('B'*128)")
R=$(gm "{\"id\":\"p1-14\",\"command\":{\"type\":\"CreateCharacter\",\"params\":{\"name\":\"$LONG128\",\"class\":\"Fighter\",\"alignment\":\"Neutral\",\"abilities\":[16,9,9,9,12,9]}}}")
check "128-char name" "ok" "$R"

# Very long name exceeding 128 chars (should fail based on max 128)
LONG200=$(python3 -c "print('C'*200)")
R=$(gm "{\"id\":\"p1-15\",\"command\":{\"type\":\"CreateCharacter\",\"params\":{\"name\":\"$LONG200\",\"class\":\"Fighter\",\"alignment\":\"Neutral\",\"abilities\":[16,9,9,9,12,9]}}}")
check "200-char name (should fail)" "err" "$R"

# Unicode character names
R=$(gm '{"id":"p1-16","command":{"type":"CreateCharacter","params":{"name":"Björk the Brave","class":"Fighter","alignment":"Neutral","abilities":[16,9,9,9,12,9]}}}')
check "Unicode name (Björk)" "ok" "$R"

R=$(gm '{"id":"p1-17","command":{"type":"CreateCharacter","params":{"name":"武士太郎","class":"Fighter","alignment":"Neutral","abilities":[16,9,9,9,12,9]}}}')
check "CJK Unicode name" "ok" "$R"

echo ""
echo "========================================"
echo "PHASE 2: Equipment Stress"
echo "========================================"

# Buy equipment (spread across characters to avoid gold exhaustion)
R=$(gm '{"id":"p2-1","command":{"type":"Buy","params":{"character":"Superman","item_name":"sword"}}}')
check "Buy sword" "ok" "$R"

R=$(gm '{"id":"p2-2","command":{"type":"Buy","params":{"character":"Superman","item_name":"plate mail"}}}')
check "Buy plate mail" "ok" "$R"

R=$(gm '{"id":"p2-3","command":{"type":"Buy","params":{"character":"Superman","item_name":"shield"}}}')
check "Buy shield" "ok" "$R"

# Equip items
R=$(gm '{"id":"p2-4","command":{"type":"Equip","params":{"character":"Superman","item_name":"sword"}}}')
check "Equip sword" "ok" "$R"

R=$(gm '{"id":"p2-5","command":{"type":"Equip","params":{"character":"Superman","item_name":"plate mail"}}}')
check "Equip plate mail" "ok" "$R"

R=$(gm '{"id":"p2-6","command":{"type":"Equip","params":{"character":"Superman","item_name":"shield"}}}')
check "Equip shield" "ok" "$R"

# Buy many torches (exact name: "Torches (6)")
for i in $(seq 1 20); do
  R=$(gm "{\"id\":\"p2-torch-$i\",\"command\":{\"type\":\"Buy\",\"params\":{\"character\":\"Filch\",\"item_name\":\"Torches (6)\"}}}")
done
check "Buy 20 sets of torches (last)" "ok" "$R"

# Equip multiple weapons (use Thorin who has gold)
R=$(gm '{"id":"p2-7","command":{"type":"Buy","params":{"character":"Thorin","item_name":"mace"}}}')
check "Buy mace (second weapon)" "ok" "$R"
R=$(gm '{"id":"p2-8","command":{"type":"Equip","params":{"character":"Thorin","item_name":"mace"}}}')
check "Equip mace (second weapon)" "ok" "$R"

# Equip multiple armor (exact name: "Leather" not "leather armour")
# Use Björk who hasn't bought anything
R=$(gm '{"id":"p2-9","command":{"type":"Buy","params":{"character":"Björk the Brave","item_name":"leather"}}}')
check "Buy leather armor" "ok" "$R"
R=$(gm '{"id":"p2-9b","command":{"type":"Buy","params":{"character":"Björk the Brave","item_name":"chainmail"}}}')
check "Buy chainmail for armor test" "ok" "$R"
R=$(gm '{"id":"p2-10","command":{"type":"Equip","params":{"character":"Björk the Brave","item_name":"leather"}}}')
check "Equip leather" "ok" "$R"
R=$(gm '{"id":"p2-10b","command":{"type":"Equip","params":{"character":"Björk the Brave","item_name":"chainmail"}}}')
check "Equip chainmail over leather" "ok" "$R"

# Unequip then re-equip
R=$(gm '{"id":"p2-11","command":{"type":"Equip","params":{"character":"Superman","item_name":"plate mail"}}}')
check "Toggle plate mail back" "ok" "$R"

# Equipment name matching test (oag-ntpgk)
# Exact DB name is "Chainmail" (one word, lowercase lookup works)
R=$(gm '{"id":"p2-12","command":{"type":"Buy","params":{"character":"Brother Cadfael","item_name":"chainmail"}}}')
check "Buy 'chainmail' (exact DB name)" "ok" "$R"

R=$(gm '{"id":"p2-13","command":{"type":"Buy","params":{"character":"Filch","item_name":"Chainmail"}}}')
check "Buy 'Chainmail' (title case)" "ok" "$R"

R=$(gm '{"id":"p2-14","command":{"type":"Buy","params":{"character":"Elrohir","item_name":"CHAINMAIL"}}}')
check "Buy 'CHAINMAIL' (uppercase)" "ok" "$R"

# List equipment (requires "params":{} even when params are optional)
R=$(gm '{"id":"p2-15","command":{"type":"ListEquipment","params":{}}}')
check "ListEquipment (no category)" "ok" "$R"

R=$(gm '{"id":"p2-16","command":{"type":"ListEquipment","params":{"category":"weapons"}}}')
check "ListEquipment weapons" "ok" "$R"

# Drop item
R=$(gm '{"id":"p2-17","command":{"type":"Drop","params":{"character":"Thorin","item_name":"mace"}}}')
check "Drop mace" "ok" "$R"

# Query encumbrance
R=$(gm '{"id":"p2-18","command":{"type":"QueryEncumbrance","params":{"character":"Grond"}}}')
check "Query Grond encumbrance" "ok" "$R"

echo ""
echo "========================================"
echo "PHASE 3: Combat Stress"
echo "========================================"

# Enter dungeon first
R=$(gm '{"id":"p3-0","command":{"type":"EnterDungeon","params":{"level":1}}}')
check "Enter dungeon" "ok" "$R"

# Large encounter: 20 goblins
R=$(gm '{"id":"p3-1","command":{"type":"SpawnEncounter","params":{"name":"goblin","count":20,"hit_dice":"1-1","ac":6,"hp":4,"damage":"1d6","morale":7,"distance":60}}}')
check "Spawn 20 goblins" "ok" "$R"

R=$(gm '{"id":"p3-2","command":{"type":"EndCombat"}}')
check "End combat (20 goblins)" "ok" "$R"

# Powerful monster: dragon
R=$(gm '{"id":"p3-3","command":{"type":"SpawnEncounter","params":{"name":"Ancient Red Dragon","count":1,"hit_dice":"11","ac":-1,"hp":60,"damage":"3d8","morale":10,"distance":120}}}')
check "Spawn dragon HD 11, AC -1, HP 60" "ok" "$R"

R=$(gm '{"id":"p3-4","command":{"type":"EndCombat"}}')
check "End combat (dragon)" "ok" "$R"

# Half-HD monsters (oag-ealzm fix)
R=$(gm '{"id":"p3-5","command":{"type":"SpawnEncounter","params":{"name":"kobold","count":5,"hit_dice":"1-1","ac":7,"hp":2,"damage":"1d4","morale":6,"distance":30}}}')
check "Spawn with HD '1-1' (half-HD)" "ok" "$R"

R=$(gm '{"id":"p3-6","command":{"type":"EndCombat"}}')
check "End combat (kobolds)" "ok" "$R"

# Zero distance combat (melee range)
R=$(gm '{"id":"p3-7","command":{"type":"SpawnEncounter","params":{"name":"spider","count":1,"hit_dice":"1","ac":8,"hp":4,"damage":"1d6","morale":7,"distance":0}}}')
check "Spawn at distance 0" "ok" "$R"

R=$(gm '{"id":"p3-7b","command":{"type":"RollInitiative"}}')
check "Initiative (distance 0)" "ok" "$R"

R=$(gm '{"id":"p3-7c","command":{"type":"Attack","params":{"character":"Grond","monster_idx":0}}}')
check "Melee attack at distance 0" "ok" "$R"

R=$(gm '{"id":"p3-8","command":{"type":"EndCombat"}}')
check "End combat (distance 0)" "ok" "$R"

# Max distance
R=$(gm '{"id":"p3-9","command":{"type":"SpawnEncounter","params":{"name":"archer","count":1,"hit_dice":"1","ac":7,"hp":5,"damage":"1d6","morale":7,"distance":240}}}')
check "Spawn at distance 240" "ok" "$R"

R=$(gm '{"id":"p3-10","command":{"type":"EndCombat"}}')
check "End combat (distance 240)" "ok" "$R"

# Negative AC
R=$(gm '{"id":"p3-11","command":{"type":"SpawnEncounter","params":{"name":"demon lord","count":1,"hit_dice":"15","ac":-3,"hp":80,"damage":"4d10","morale":12,"distance":60}}}')
check "Spawn with AC -3" "ok" "$R"

R=$(gm '{"id":"p3-12","command":{"type":"EndCombat"}}')
check "End combat (AC -3)" "ok" "$R"

# Full combat sequence with Close then melee attack
R=$(gm '{"id":"p3-13","command":{"type":"SpawnEncounter","params":{"name":"orc","count":5,"hit_dice":"1","ac":6,"hp":4,"damage":"1d6","morale":8,"distance":30}}}')
check "Spawn 5 orcs for combat test" "ok" "$R"

R=$(gm '{"id":"p3-14","command":{"type":"RollInitiative"}}')
check "Roll initiative" "ok" "$R"

# Close to melee range first
R=$(gm '{"id":"p3-close","command":{"type":"Close","params":{"character":"Grond"}}}')
check "Grond closes to melee" "ok" "$R"

# Each character attacks one orc (one attack per character per round)
R=$(gm '{"id":"p3-atk-0","command":{"type":"Attack","params":{"character":"Grond","monster_idx":0}}}')
check "Grond attacks orc 0" "ok" "$R"
R=$(gm '{"id":"p3-atk-1","command":{"type":"Attack","params":{"character":"Thorin","monster_idx":1}}}')
check "Thorin attacks orc 1" "ok" "$R"
R=$(gm '{"id":"p3-atk-2","command":{"type":"Attack","params":{"character":"Superman","monster_idx":2}}}')
check "Superman attacks orc 2" "ok" "$R"
R=$(gm '{"id":"p3-atk-3","command":{"type":"Attack","params":{"character":"Filch","monster_idx":3}}}')
check "Filch attacks orc 3" "ok" "$R"
R=$(gm '{"id":"p3-atk-4","command":{"type":"Attack","params":{"character":"Weakling","monster_idx":4}}}')
check "Weakling attacks orc 4" "ok" "$R"

# Verify "already acted" restriction
R=$(gm '{"id":"p3-atk-dup","command":{"type":"Attack","params":{"character":"Grond","monster_idx":1}}}')
check "Grond second attack rejected (already acted)" "err" "$R"

# Monster attacks party members (try each index, accept first success)
# Combat is non-deterministic: some orcs may have died from party attacks
MONSTER_HIT=false
for i in 4 3 2 1 0; do
  R=$(gm "{\"id\":\"p3-matk-$i\",\"command\":{\"type\":\"MonsterAttack\",\"params\":{\"monster_idx\":$i,\"character\":\"Superman\"}}}")
  S=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin).get('success',''))" 2>/dev/null)
  if [ "$S" = "True" ]; then
    PASS=$((PASS+1))
    echo "  PASS: Orc $i attacks Superman"
    MONSTER_HIT=true
    break
  fi
done
if [ "$MONSTER_HIT" = "false" ]; then
  FAIL=$((FAIL+1))
  echo "  FAIL: No living orc could attack"
fi

R=$(gm '{"id":"p3-17","command":{"type":"CheckMorale"}}')
check "Check morale" "ok" "$R"

R=$(gm '{"id":"p3-18","command":{"type":"EndCombat"}}')
check "End combat (orcs)" "ok" "$R"

# Test healing dead character (oag-c4bni fix)
R=$(gm '{"id":"p3-19","command":{"type":"Damage","params":{"character":"Pippin","amount":100}}}')
check "Damage Pippin to death" "ok" "$R"

R=$(gm '{"id":"p3-20","command":{"type":"Heal","params":{"character":"Pippin","amount":5}}}')
check "Heal dead Pippin (should reject)" "err" "$R"

# SetHp on dead character
R=$(gm '{"id":"p3-21","command":{"type":"SetHp","params":{"character":"Pippin","hp":5}}}')
check "SetHp on dead Pippin" "ok" "$R"

# AwardXp to dead character
R=$(gm '{"id":"p3-22","command":{"type":"Damage","params":{"character":"Pippin","amount":100}}}')
check "Kill Pippin again" "ok" "$R"

R=$(gm '{"id":"p3-23","command":{"type":"AwardXp","params":{"character":"Pippin","xp":100}}}')
check "AwardXp to dead Pippin (should reject)" "err" "$R"

echo ""
echo "========================================"
echo "PHASE 4: Multi-Encounter Chain"
echo "========================================"

# Fight 1: Goblins at distance 0 (melee)
R=$(gm '{"id":"p4-1","command":{"type":"SpawnEncounter","params":{"name":"goblin","count":3,"hit_dice":"1-1","ac":6,"hp":3,"damage":"1d6","morale":7,"distance":0}}}')
check "Encounter 1: 3 goblins (melee)" "ok" "$R"

R=$(gm '{"id":"p4-2","command":{"type":"RollInitiative"}}')
check "Initiative (goblins)" "ok" "$R"
for i in 0 1 2; do
  R=$(gm "{\"id\":\"p4-atk1-$i\",\"command\":{\"type\":\"Attack\",\"params\":{\"character\":\"Grond\",\"monster_idx\":$i}}}")
done
R=$(gm '{"id":"p4-3","command":{"type":"EndCombat"}}')
check "End combat 1" "ok" "$R"

# Record HP after fight 1
R=$(gm '{"id":"p4-hp1","command":{"type":"QueryParty"}}')
check "Query party after fight 1" "ok" "$R"
echo "  Party HP after fight 1:"
echo "$R" | python3 -c "
import sys, json
d = json.load(sys.stdin)
members = d.get('data',{}).get('members',[])
for m in members:
    print(f\"    {m.get('name','?')}: {m.get('hp','?')}/{m.get('max_hp','?')}\")
" 2>/dev/null || echo "  (parse error)"

# Fight 2: Orcs
R=$(gm '{"id":"p4-4","command":{"type":"SpawnEncounter","params":{"name":"orc","count":2,"hit_dice":"1","ac":6,"hp":4,"damage":"1d6","morale":8,"distance":0}}}')
check "Encounter 2: 2 orcs" "ok" "$R"
R=$(gm '{"id":"p4-5","command":{"type":"EndCombat"}}')
check "End combat 2" "ok" "$R"

# Fight 3: Skeletons + Turn Undead
R=$(gm '{"id":"p4-6","command":{"type":"SpawnEncounter","params":{"name":"skeleton","count":5,"hit_dice":"1","ac":7,"hp":4,"damage":"1d6","morale":12,"distance":0}}}')
check "Encounter 3: 5 skeletons" "ok" "$R"

R=$(gm '{"id":"p4-7","command":{"type":"TurnUndead","params":{"character":"Brother Cadfael","monster_idx":0}}}')
check "Turn Undead (skeletons)" "ok" "$R"

R=$(gm '{"id":"p4-8","command":{"type":"EndCombat"}}')
check "End combat 3" "ok" "$R"

# Fight 4: Ogre, full fight at melee
R=$(gm '{"id":"p4-9","command":{"type":"SpawnEncounter","params":{"name":"ogre","count":1,"hit_dice":"4+1","ac":5,"hp":20,"damage":"1d10","morale":10,"distance":0}}}')
check "Encounter 4: 1 ogre" "ok" "$R"

R=$(gm '{"id":"p4-10","command":{"type":"RollInitiative"}}')
check "Initiative (ogre)" "ok" "$R"

# Multiple characters attack (use characters likely to be alive)
R=$(gm '{"id":"p4-11","command":{"type":"Attack","params":{"character":"Superman","monster_idx":0}}}')
check "Superman attacks ogre" "ok" "$R"
R=$(gm '{"id":"p4-12","command":{"type":"Attack","params":{"character":"Thorin","monster_idx":0}}}')
check "Thorin attacks ogre" "ok" "$R"
R=$(gm '{"id":"p4-13","command":{"type":"Attack","params":{"character":"Björk the Brave","monster_idx":0}}}')
check "Björk attacks ogre" "ok" "$R"

R=$(gm '{"id":"p4-14","command":{"type":"EndCombat"}}')
check "End combat 4" "ok" "$R"

# Verify HP didn't reset between encounters
R=$(gm '{"id":"p4-xp","command":{"type":"QueryParty"}}')
check "Query party after 4 fights" "ok" "$R"
echo "  Party state after 4 encounters:"
echo "$R" | python3 -c "
import sys, json
d = json.load(sys.stdin)
members = d.get('data',{}).get('members',[])
for m in members:
    print(f\"    {m.get('name','?')}: {m.get('xp',0)} XP, HP {m.get('hp','?')}/{m.get('max_hp','?')}\")
" 2>/dev/null || echo "  (parse error)"

echo ""
echo "========================================"
echo "PHASE 5: Spell Slot Management"
echo "========================================"

# Spell lookups
R=$(gm '{"id":"p5-1","command":{"type":"LookupSpell","params":{"name":"Magic Missile"}}}')
check "Lookup Magic Missile" "ok" "$R"

R=$(gm '{"id":"p5-2","command":{"type":"LookupSpell","params":{"name":"Sleep"}}}')
check "Lookup Sleep" "ok" "$R"

# Spell casting test - KNOWN BUG: initiative clears declarations (oag-7sa5i)
R=$(gm '{"id":"p5-3","command":{"type":"SpawnEncounter","params":{"name":"rat","count":3,"hit_dice":"1-1","ac":9,"hp":1,"damage":"1d3","morale":5,"distance":30}}}')
check "Spawn rats for spell test" "ok" "$R"

R=$(gm '{"id":"p5-4","command":{"type":"DeclareSpell","params":{"character":"Mystara","spell":"Sleep"}}}')
check "Declare Sleep" "ok" "$R"

R=$(gm '{"id":"p5-5","command":{"type":"RollInitiative"}}')
check "Initiative for spell combat" "ok" "$R"

# KNOWN BUG: CastSpell will fail because RollInitiative cleared declarations
R=$(gm '{"id":"p5-6","command":{"type":"CastSpell","params":{"character":"Mystara"}}}')
S=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin).get('success',''))" 2>/dev/null)
if [ "$S" = "True" ]; then
  PASS=$((PASS+1))
  echo "  PASS: Cast Sleep (bug oag-7sa5i was fixed!)"
else
  known_bug "CastSpell fails after Init (Declare→Init→Cast)" "oag-7sa5i"
fi

R=$(gm '{"id":"p5-7","command":{"type":"EndCombat"}}')
check "End spell combat" "ok" "$R"

# Spell slot tracking test - KNOWN BUG: no slot tracking (oag-t8467)
R=$(gm '{"id":"p5-8","command":{"type":"SpawnEncounter","params":{"name":"rat","count":2,"hit_dice":"1-1","ac":9,"hp":1,"damage":"1d3","morale":5,"distance":30}}}')
check "Spawn more rats" "ok" "$R"

R=$(gm '{"id":"p5-9","command":{"type":"DeclareSpell","params":{"character":"Mystara","spell":"Sleep"}}}')
S=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin).get('success',''))" 2>/dev/null)
if [ "$S" = "False" ]; then
  PASS=$((PASS+1))
  echo "  PASS: DeclareSpell rejected (slots exhausted - bug oag-t8467 was fixed!)"
else
  known_bug "DeclareSpell succeeds without available slots" "oag-t8467"
fi

R=$(gm '{"id":"p5-10","command":{"type":"EndCombat"}}')
check "End combat" "ok" "$R"

# Rest
R=$(gm '{"id":"p5-11","command":{"type":"Rest"}}')
check "Rest" "ok" "$R"

echo ""
echo "========================================"
echo "PHASE 6: Treasure Generation"
echo "========================================"

# Roll treasure for each type
for letter in A B C D E F G H I J K L M N O P Q R S T U V; do
  R=$(gm "{\"id\":\"p6-roll-$letter\",\"command\":{\"type\":\"RollTreasure\",\"params\":{\"letter\":\"$letter\"}}}")
  check "RollTreasure type $letter" "ok" "$R"
done

# Lookup treasure types
for letter in A B C D E; do
  R=$(gm "{\"id\":\"p6-look-$letter\",\"command\":{\"type\":\"LookupTreasureType\",\"params\":{\"letter\":\"$letter\"}}}")
  check "LookupTreasureType $letter" "ok" "$R"
done

# Search magic items
R=$(gm '{"id":"p6-s1","command":{"type":"SearchItems","params":{"query":"sword"}}}')
check "SearchItems 'sword'" "ok" "$R"

R=$(gm '{"id":"p6-s2","command":{"type":"SearchItems","params":{"query":"potion"}}}')
check "SearchItems 'potion'" "ok" "$R"

# Lookup specific items
R=$(gm '{"id":"p6-l1","command":{"type":"LookupItem","params":{"name":"Sword +1"}}}')
check "LookupItem 'Sword +1'" "ok" "$R"

R=$(gm '{"id":"p6-l2","command":{"type":"LookupItem","params":{"name":"Ring of Invisibility"}}}')
check "LookupItem 'Ring of Invisibility'" "ok" "$R"

echo ""
echo "========================================"
echo "PHASE 7: Save/Load Stress"
echo "========================================"

# Normal save
R=$(gm '{"id":"p7-1","command":{"type":"Save","params":{"path":"test_save.json"}}}')
check "Save normal" "ok" "$R"

# Very long filename
LONGNAME=$(python3 -c "print('a'*200 + '.json')")
R=$(gm "{\"id\":\"p7-2\",\"command\":{\"type\":\"Save\",\"params\":{\"path\":\"$LONGNAME\"}}}")
check "Save with very long filename" "ok" "$R"

# Path traversal (security)
R=$(gm '{"id":"p7-3","command":{"type":"Save","params":{"path":"../../../tmp/evil.json"}}}')
check "Path traversal rejected" "err" "$R"

R=$(gm '{"id":"p7-4","command":{"type":"Save","params":{"path":"../../etc/passwd"}}}')
check "Path traversal to /etc/passwd rejected" "err" "$R"

# Load nonexistent file
R=$(gm '{"id":"p7-5","command":{"type":"Load","params":{"path":"nonexistent_file_xyz.json"}}}')
check "Load nonexistent file (graceful error)" "err" "$R"

# Save and reload
R=$(gm '{"id":"p7-6","command":{"type":"Save","params":{"path":"stress_test_save.json"}}}')
check "Save for reload test" "ok" "$R"

R=$(gm '{"id":"p7-7","command":{"type":"Load","params":{"path":"stress_test_save.json"}}}')
check "Load saved game" "ok" "$R"

# Path with null bytes
R=$(gm '{"id":"p7-8","command":{"type":"Save","params":{"path":"evil\u0000file.json"}}}')
check "Save with null bytes rejected" "err" "$R"

echo ""
echo "========================================"
echo "PHASE 8: Dice Rolling Edge Cases"
echo "========================================"

# Valid rolls
R=$(gm '{"id":"p8-1","command":{"type":"Roll","params":{"notation":"3d6"}}}')
check "Roll 3d6" "ok" "$R"

R=$(gm '{"id":"p8-2","command":{"type":"Roll","params":{"notation":"1d20+5"}}}')
check "Roll 1d20+5" "ok" "$R"

R=$(gm '{"id":"p8-3","command":{"type":"Roll","params":{"notation":"1d20-3"}}}')
check "Roll 1d20-3" "ok" "$R"

R=$(gm '{"id":"p8-4","command":{"type":"Roll","params":{"notation":"2d6+1"}}}')
check "Roll 2d6+1" "ok" "$R"

R=$(gm '{"id":"p8-5","command":{"type":"Roll","params":{"notation":"d%"}}}')
check "Roll d%" "ok" "$R"

R=$(gm '{"id":"p8-6","command":{"type":"Roll","params":{"notation":"1d100"}}}')
check "Roll 1d100" "ok" "$R"

R=$(gm '{"id":"p8-7","command":{"type":"Roll","params":{"notation":"10d10"}}}')
check "Roll 10d10" "ok" "$R"

R=$(gm '{"id":"p8-8","command":{"type":"Roll","params":{"notation":"1d4"}}}')
check "Roll 1d4" "ok" "$R"

# Error cases
R=$(gm '{"id":"p8-9","command":{"type":"Roll","params":{"notation":"0d6"}}}')
check "Roll 0d6 (error)" "err" "$R"

R=$(gm '{"id":"p8-10","command":{"type":"Roll","params":{"notation":"1d0"}}}')
check "Roll 1d0 (error)" "err" "$R"

R=$(gm '{"id":"p8-11","command":{"type":"Roll","params":{"notation":"-1d6"}}}')
check "Roll -1d6 (error)" "err" "$R"

R=$(gm '{"id":"p8-12","command":{"type":"Roll","params":{"notation":"abc"}}}')
check "Roll abc (error)" "err" "$R"

R=$(gm '{"id":"p8-13","command":{"type":"Roll","params":{"notation":""}}}')
check "Roll empty string (error)" "err" "$R"

R=$(gm '{"id":"p8-14","command":{"type":"Roll","params":{"notation":"999d999"}}}')
check "Roll 999d999 (extreme)" "ok" "$R"

R=$(gm '{"id":"p8-15","command":{"type":"Roll","params":{"notation":"1d6+999"}}}')
check "Roll 1d6+999" "ok" "$R"

echo ""
echo "========================================"
echo "PHASE 9: API Protocol Stress"
echo "========================================"

# Test /health endpoint
R=$(curl -s "http://127.0.0.1:${PORT}/health")
if [ "$R" = "ok" ]; then
  PASS=$((PASS+1))
  echo "  PASS: Health endpoint returns 'ok'"
else
  FAIL=$((FAIL+1))
  echo "  FAIL: Health endpoint: $R"
fi

# 100KB body exceeds 64KB limit - should be rejected
LARGE_BODY=$(python3 -c "import json; print(json.dumps({'id':'p9-big','command':{'type':'Roll','params':{'notation':'1d6'}},'extra':'x'*100000}))")
R=$(curl -s -X POST "http://127.0.0.1:${PORT}/api/v1/gm" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$LARGE_BODY" 2>/dev/null)
check "100KB body rejected (64KB limit)" "err" "$R"

# Test HTTP methods against /api/v1/gm
R=$(curl -s -o /dev/null -w "%{http_code}" -X GET "http://127.0.0.1:${PORT}/api/v1/gm")
if [ "$R" = "405" ]; then
  PASS=$((PASS+1))
  echo "  PASS: GET /api/v1/gm returns 405"
else
  echo "  INFO: GET /api/v1/gm returns $R (expected 405)"
fi

R=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "http://127.0.0.1:${PORT}/api/v1/gm")
echo "  INFO: PUT /api/v1/gm returns $R"

R=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "http://127.0.0.1:${PORT}/api/v1/gm")
echo "  INFO: DELETE /api/v1/gm returns $R"

# Missing auth header
R=$(curl -s -X POST "http://127.0.0.1:${PORT}/api/v1/gm" \
  -H "Content-Type: application/json" \
  -d '{"id":"p9-noauth","command":{"type":"QueryMode"}}')
check "Request without auth (should reject)" "err" "$R"

# Bad JSON
R=$(curl -s -X POST "http://127.0.0.1:${PORT}/api/v1/gm" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d 'not valid json at all')
check "Bad JSON body" "err" "$R"

# Empty body
R=$(curl -s -X POST "http://127.0.0.1:${PORT}/api/v1/gm" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '')
check "Empty body" "err" "$R"

# Duplicate JSON keys - serde_json rejects by default, that's correct
R=$(curl -s -X POST "http://127.0.0.1:${PORT}/api/v1/gm" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":"p9-dup","id":"p9-dup2","command":{"type":"QueryMode"}}')
check "Duplicate JSON keys rejected" "err" "$R"

# Binary data
R=$(curl -s -X POST "http://127.0.0.1:${PORT}/api/v1/gm" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  --data-binary $'\x00\x01\x02\x03\xff\xfe')
check "Binary data body" "err" "$R"

echo ""
echo "========================================"
echo "PHASE 10: Full Game Lifecycle"
echo "========================================"

# Save starting state
R=$(gm '{"id":"p10-save1","command":{"type":"Save","params":{"path":"lifecycle_start.json"}}}')
check "Save starting state" "ok" "$R"

# Equip the party
R=$(gm '{"id":"p10-eq1","command":{"type":"Buy","params":{"character":"Brother Cadfael","item_name":"mace"}}}')
check "Buy Cadfael mace" "ok" "$R"
R=$(gm '{"id":"p10-eq2","command":{"type":"Equip","params":{"character":"Brother Cadfael","item_name":"mace"}}}')
check "Equip mace on Cadfael" "ok" "$R"

# Light a torch before exploring (dungeon is dark!)
R=$(gm '{"id":"p10-light","command":{"type":"Light","params":{"source":"torch","carrier":"Superman"}}}')
check "Light torch" "ok" "$R"

# Query exploration to find current room
R=$(gm '{"id":"p10-qe","command":{"type":"QueryExploration"}}')
check "Query exploration state" "ok" "$R"
CURRENT_ROOM=$(echo "$R" | python3 -c "
import sys, json
d = json.load(sys.stdin)
data = d.get('data', {})
print(data.get('current_room', data.get('room_id', 1)))
" 2>/dev/null || echo "1")
echo "  Current room: $CURRENT_ROOM"

# Add a new room and door from current room
R=$(gm '{"id":"p10-r1","command":{"type":"AddRoom","params":{"id":99,"name":"Treasure Room"}}}')
check "Add treasure room (id=99)" "ok" "$R"

# Connect current room to new room
R=$(gm "{\"id\":\"p10-d1\",\"command\":{\"type\":\"AddDoor\",\"params\":{\"id\":99,\"room_a\":$CURRENT_ROOM,\"room_b\":99}}}")
check "Add door from current room to treasure room" "ok" "$R"

# Force door open and move through
R=$(gm '{"id":"p10-force","command":{"type":"ForceDoor","params":{"door_id":99,"character":"Superman"}}}')
check "Force door open" "ok" "$R"

R=$(gm '{"id":"p10-m1","command":{"type":"MoveRoom","params":{"door_id":99}}}')
check "Move to treasure room" "ok" "$R"

R=$(gm '{"id":"p10-search","command":{"type":"Search","params":{"is_elf":false}}}')
check "Search treasure room" "ok" "$R"

# Fight encounter at melee range
R=$(gm '{"id":"p10-fight","command":{"type":"SpawnEncounter","params":{"name":"skeleton","count":2,"hit_dice":"1","ac":7,"hp":4,"damage":"1d6","morale":12,"distance":0}}}')
check "Encounter in treasure room" "ok" "$R"

R=$(gm '{"id":"p10-init","command":{"type":"RollInitiative"}}')
check "Initiative" "ok" "$R"

# Use living characters (Grond may have died in earlier fights)
R=$(gm '{"id":"p10-atk1","command":{"type":"Attack","params":{"character":"Superman","monster_idx":0}}}')
check "Superman attacks skeleton 0" "ok" "$R"

R=$(gm '{"id":"p10-atk2","command":{"type":"Attack","params":{"character":"Thorin","monster_idx":1}}}')
check "Thorin attacks skeleton 1" "ok" "$R"

R=$(gm '{"id":"p10-end","command":{"type":"EndCombat"}}')
check "End combat" "ok" "$R"

# Loot command requires lootable items in the room; roll treasure instead
R=$(gm '{"id":"p10-treasure","command":{"type":"RollTreasure","params":{"letter":"B"}}}')
check "Roll treasure type B" "ok" "$R"

# Award XP and level up (use Superman who is alive)
R=$(gm '{"id":"p10-xp1","command":{"type":"AwardXp","params":{"character":"Superman","xp":500}}}')
check "Award 500 XP to Superman" "ok" "$R"

R=$(gm '{"id":"p10-xp2","command":{"type":"AwardXp","params":{"character":"Superman","xp":2000}}}')
check "Award 2000 more XP" "ok" "$R"

R=$(gm '{"id":"p10-lvl","command":{"type":"LevelUp","params":{"character":"Superman"}}}')
check "Level up Superman" "ok" "$R"

# Save mid-dungeon
R=$(gm '{"id":"p10-save2","command":{"type":"Save","params":{"path":"mid_dungeon.json"}}}')
check "Save mid-dungeon" "ok" "$R"

# Transition: dungeon → wilderness
R=$(gm '{"id":"p10-wild","command":{"type":"EnterWilderness","params":{"terrain":"forest"}}}')
check "Enter wilderness (from dungeon)" "ok" "$R"

# Wilderness travel
R=$(gm '{"id":"p10-hex1","command":{"type":"AddHex","params":{"x":1,"y":0,"terrain":"forest"}}}')
check "Add forest hex" "ok" "$R"

R=$(gm '{"id":"p10-hex2","command":{"type":"AddHex","params":{"x":2,"y":0,"terrain":"hills"}}}')
check "Add hills hex" "ok" "$R"

R=$(gm '{"id":"p10-trav1","command":{"type":"Travel","params":{"x":1,"y":0}}}')
check "Travel to forest hex" "ok" "$R"

# Forage
R=$(gm '{"id":"p10-forage","command":{"type":"Forage"}}')
check "Forage in forest" "ok" "$R"

# Save → Load → Continue
R=$(gm '{"id":"p10-save3","command":{"type":"Save","params":{"path":"wilderness_save.json"}}}')
check "Save in wilderness" "ok" "$R"

R=$(gm '{"id":"p10-load","command":{"type":"Load","params":{"path":"wilderness_save.json"}}}')
check "Load wilderness save" "ok" "$R"

R=$(gm '{"id":"p10-trav2","command":{"type":"Travel","params":{"x":2,"y":0}}}')
check "Travel after load" "ok" "$R"

echo ""
echo "========================================"
echo "EDGE CASES: Wrong mode commands"
echo "========================================"

# Commands in wrong mode (we're in wilderness now)
R=$(gm '{"id":"ec-1","command":{"type":"AdvanceTurn","params":{}}}')
check "AdvanceTurn in wilderness (wrong mode)" "err" "$R"

# Search works in wilderness too (searching the hex) - valid behavior
R=$(gm '{"id":"ec-2","command":{"type":"Search","params":{"is_elf":false}}}')
check "Search in wilderness" "ok" "$R"

R=$(gm '{"id":"ec-3","command":{"type":"Attack","params":{"character":"Grond","monster_idx":0}}}')
check "Attack outside combat (wrong mode)" "err" "$R"

R=$(gm '{"id":"ec-4","command":{"type":"RollInitiative"}}')
check "RollInitiative outside combat (wrong mode)" "err" "$R"

# Commands with missing required params
R=$(gm '{"id":"ec-5","command":{"type":"CreateCharacter","params":{}}}')
check "CreateCharacter no params" "err" "$R"

R=$(gm '{"id":"ec-6","command":{"type":"Attack","params":{}}}')
check "Attack no params" "err" "$R"

R=$(gm '{"id":"ec-7","command":{"type":"Buy","params":{}}}')
check "Buy no params" "err" "$R"

# Commands with wrong types
R=$(gm '{"id":"ec-8","command":{"type":"Roll","params":{"notation":12345}}}')
check "Roll with number instead of string" "err" "$R"

# Unknown command type
R=$(gm '{"id":"ec-9","command":{"type":"FlyToTheMoon"}}')
check "Unknown command type" "err" "$R"

# Missing command type
R=$(gm '{"id":"ec-10","command":{}}')
check "Missing command type" "err" "$R"

# QueryMode is a unit variant - no params field allowed (correct protocol behavior)
R=$(gm '{"id":"ec-11","command":{"type":"QueryMode"}}')
check "QueryMode (unit variant, no params)" "ok" "$R"

echo ""
echo "========================================"
echo "SUMMARY"
echo "========================================"
echo "PASSED:     $PASS"
echo "FAILED:     $FAIL"
echo "KNOWN_BUGS: $KNOWN"
echo "TOTAL:      $((PASS + FAIL + KNOWN))"

if [ $FAIL -gt 0 ]; then
  echo ""
  echo "UNEXPECTED FAILURES:"
  echo -e "$BUGS"
fi

if [ $KNOWN -gt 0 ]; then
  echo ""
  echo "Known bugs confirmed (filed as beads):"
  echo "  oag-7sa5i: RollInitiative clears spell declarations"
  echo "  oag-t8467: No spell slot tracking"
fi

echo ""
echo "Stress test complete."
