# The brain's map

Every file of `treadcore-brain` and every function in it, with the
first sentence of its own doc comment. Written by
`scripts/brainmap.py` (`make brainmap`); do not edit by hand. The
design and its review are `LAB-REPORT-BRAIN.md`; the review of the
structure as built is `checkins/`.

17 files, 360 functions (121 with no doc comment), 81 tests.

| File | Lines | Functions | Undocumented | Tests | What it is |
| --- | --- | --- | --- | --- | --- |
| [`board.rs`](#boardrs) | 636 | 22 | 8 | 6 | The team board (roadmap step 12e, LAB-REPORT-BRAIN.md sections 5 and 9). |
| [`commander.rs`](#commanderrs) | 3866 | 72 | 23 | 23 | The Commander (roadmap step 11a): a Brain for the player's own seat. |
| [`crew.rs`](#crewrs) | 502 | 16 | 10 | 0 | The fleet commander and its crews (Paul, 16 October: "each side has a fleet commander and they request and organize fort construction"). |
| [`cycle.rs`](#cyclers) | 438 | 9 | 3 | 4 | The base cycle (roadmap step 12f1, LAB-REPORT-BRAIN.md section 6.1; Paul, 18 October: "after they fill up on base resources, add to the base 1 item, 1 improvement, and attempt to build roads to other established bases... |
| [`facts.rs`](#factsrs) | 212 | 10 | 2 | 2 | What the map says, worked out once (roadmap steps 12b and 12f2, LAB-REPORT-BRAIN.md sections 5 and 6.2). |
| [`fleet.rs`](#fleetrs) | 1172 | 20 | 2 | 8 | Fleets (Paul, 15 October: "can we start a game with ... 3 teams of base building, 5 on each, and 1 random aggro bot"): sixteen seats dealt into three teams of five neighbours round the map's edge and one RuleBot alone... |
| [`fortify.rs`](#fortifyrs) | 676 | 25 | 3 | 2 | Fortification to a plan (roadmap step 11b): a castle's gate. |
| [`hop.rs`](#hoprs) | 873 | 24 | 4 | 0 | Island hopping: a bot that has taken everything it can reach goes for what it cannot, across the water. |
| [`intent.rs`](#intentrs) | 181 | 9 | 5 | 3 | Holding a decision (roadmap step 12d, LAB-REPORT-BRAIN.md section 7). |
| [`ledger.rs`](#ledgerrs) | 322 | 11 | 4 | 3 | The ledger (roadmap step 12g, LAB-REPORT-BRAIN.md section 8). |
| [`lib.rs`](#librs) | 2534 | 55 | 21 | 17 | treadcore-brain -- the controller interface, and one controller. |
| [`memory.rs`](#memoryrs) | 96 | 10 | 6 | 1 | Memories that run out (roadmap step 12b, finding L6 of LAB-REPORT-BRAIN.md). |
| [`meter.rs`](#meterrs) | 261 | 20 | 8 | 2 | What the brain spends (roadmap step 12a, LAB-REPORT-BRAIN.md section 5, "measure first"). |
| [`rotation.rs`](#rotationrs) | 271 | 7 | 0 | 2 | Rotation (Paul, 17 October: "every bot should have a push point, and a defense point. |
| [`status.rs`](#statusrs) | 352 | 9 | 5 | 1 | Status as codes (roadmap step 12b, finding L3 of LAB-REPORT-BRAIN.md, section 22). |
| [`tactics.rs`](#tacticsrs) | 676 | 16 | 3 | 3 | Fleet tactics (Paul, 16 October: "we want a bunch of scalars and reasons to collaborative bot behavior. |
| [`weights.rs`](#weightsrs) | 495 | 25 | 14 | 4 | Weights and capabilities (roadmap step 12c, LAB-REPORT-BRAIN.md sections 4 and 5). |

## board.rs

The team board (roadmap step 12e, LAB-REPORT-BRAIN.md sections 5 and 9). One per team, shared by its Commanders (`Shared`), so the team thinks on one picture of the map instead of each tank working the same things out alone from its own copy (finding L5):  - the influence map: 16 x 16 sectors of 8 x 8 tiles, ours minus theirs, each tank by its worth in a fight (armour, shells) and each pillbox by `pillbox_worth`, falling off with distance; the front is where it changes sign, territory where it is ours; - the defence points: the side's pillboxes and depots nearest the threat, two to four; - the members: where each is, how ready (armour times shells), what it is doing, for the coordination screen (11c's allies' goals); - the jobs the fleet commander gave out, kept, so a crew that missed the one tick of its `Requested` still finds its job; - the posture: what the team should mostly be doing; - rotation across the team: a hurt tank at the hottest point is swapped for a fresh one at a cold point, confirmed over two rebuilds (`TEAM_ROTATE`); - the part-laid bridges (`BRIDGE_JOBS`, LR 6.2): each bridge a member lays is posted with its water tiles still to lay; left part-laid (the layer gave up, went home, died), it stays a job for the next teammate with material, until laid or untouched for `bridge_keep`.  Rebuilt once a second by whichever member finds it due first; bots run in seat order every tick, so that is the same member in every replay. A Mutex because a Brain is Send + Sync; nobody waits on it.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 36 | `shared` |  | yes |  |
| 86 | `word` | Posture | yes |  |
| 121 | `of` | PostureScores | yes | The score of one posture. |
| 179 | `worth` |  |  |  |
| 188 | `readiness` |  | yes | A tank's readiness: armour times shells, as shares of full. |
| 196 | `sector_of` |  | yes |  |
| 202 | `sector_centre` |  | yes |  |
| 209 | `post` | TeamBoard | yes | Every think of a member: where it is and what it is doing, and the jobs it heard given out. |
| 235 | `post_bridge` | TeamBoard | yes | A member laying a bridge says so every think: the same bridge (a tile or the target in common) is updated, a new one posted. |
| 253 | `prune_bridges` | TeamBoard | yes | The tiles laid since, by anyone, off the jobs; a bridge laid, or untouched for `keep`, off the board. |
| 266 | `take_bridge` | TeamBoard | yes | A part-laid bridge nobody is laying, its next tile within `reach` of `at`, that `ok` takes: the nearest, now `p`'s. |
| 277 | `due` | TeamBoard | yes |  |
| 282 | `job_for` | TeamBoard | yes | The job given to `p` lately, if any. |
| 287 | `order_for` | TeamBoard | yes | What the board orders `p` now, if anything. |
| 292 | `done` | TeamBoard | yes | The order is carried out (home and full, or relieved): drop it. |
| 296 | `influence_at` | TeamBoard | yes |  |
| 300 | `threat_at` | TeamBoard | yes |  |
| 305 | `front` | TeamBoard | yes | The front: our sectors with a neighbour that is theirs. |
| 321 | `territory` | TeamBoard | yes | The share of the map that is ours. |
| 329 | `rebuild` | TeamBoard | yes | Once a second: the maps, the points, the posture, the swaps. |
| 432 | `rotate` | TeamBoard |  | Rotation across the team (LR 9): the hottest point's least ready member against the most ready member elsewhere. |
| 471 | `placed` |  |  | Four seats, two teams (0 and 2, 1 and 3), placed; every tank seen. |

Tests (6):

- `the_team_defends_a_threat_at_the_base_not_far_off` (line 488): Defend is a threat at the base, not the map's hostile tanks summed (LR 24): eight of them 80 tiles off read 0.73 on the sector threat, over its 0.5, and sent the team home in 94% of a match's picks.
- `each_posture_scores_its_own_case` (line 510): Each posture wins on its own case (LR 25), and `posture_scored` 0 is the threshold ladder in every case.
- `the_front_is_where_influence_crosses_zero` (line 560)
- `a_hurt_tank_swaps_out` (line 576)
- `a_part_laid_bridge_waits_for_the_next_tank` (line 596)
- `a_missed_job_is_still_on_the_board` (line 625)

## commander.rs

The Commander (roadmap step 11a): a Brain for the player's own seat.  A person commands the tank with a goal instead of driving it: hunt pillboxes, defend a pillbox, defend a depot, escort an ally, or hold. The Commander drives, aims and sends the squad toward the goal; a RuleBot inside it keeps the doctrine it already has for drones and for shooting at tanks, and the Commander keeps only those of its orders that serve the goal. It sees what the player sees and its orders are validated like a keyboard's, so a goal is never a cheat, and the world, the wire and the replay know nothing of it.  The pillbox, by the numbers: 60 health, 8 tiles of reach, 15 every two seconds, at tanks and drones only, never at a squad. The direct gun reaches 10 and does 45. So a live hostile pillbox dies to two direct shots from the ring between 8 and 10 tiles, taking none back; then the squad stands on it for ten seconds and it is ours. A neutral one does not have to be shot at all: the squad takes it live while the tank stays out of its ring.  Auto (step 11c's hybrid): a ladder of rungs, each with a condition, tried top down every tick; the first whose condition holds is what the tank does, and the player turns rungs on and off:  1. defend base: a hostile tank within BASE_ALARM of the base, or a pillbox of the base shot at. The tank fights it under the base's guns, a few tiles out toward it; the squad mends between shots. 2. fortify: the castle round the base is not yet standing. 3. turrets: the castle stands and nothing has come for QUIET; bring the side's outlying pillboxes home, one at a time, and set them down in the bastions at the wall's ends (step 11h). 4. hunt: a pillbox not of my side within EXPAND_R of the base. 5. expand: a depot not of my side, the nearest the base first (its guard pillboxes shot dead first, then the tank sits on it), else a pillbox anywhere. A side that only hunted pillboxes held three depots for fifteen minutes while dead teams held five each. 6. guard: on guard at the base, mending.  Low on shells or armour, the tank goes home first whatever the rung.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 127 | `label` | Rung | yes |  |
| 137 | `index` | Rung | yes |  |
| 146 | `leaned` | Rung | yes | The rungs a team posture leans toward (LR 24). |
| 159 | `pick` | Rung | yes | The rung to take: of those switched on that hold, the best by `(1 - w) order + w lean`, where order runs from 1 for the first rung down to 0 for the last and lean is 1 for a rung the posture leans toward. |
| 207 | `label` | Goal | yes |  |
| 444 | `wingman` | Commander | yes | A bot seat that follows `leader` (step 11a after Paul's notes): an aggressive, greedy wingman that also runs its own alliances. |
| 451 | `new` | Commander | yes |  |
| 459 | `set_tactics` | Commander | yes | The weights for its fights (tactics.rs); None shoots the nearest and answers every ping, as before them. |
| 465 | `set_settings` | Commander | yes | The weights and capabilities: the tactics from the weights, each part of the brain on or off by its bit (weights.rs, roadmap 12c). |
| 518 | `caps` | Commander |  |  |
| 523 | `set_goal` | Commander | yes | A new goal: what was half done for the old one is dropped. |
| 537 | `mine_or_allied` | Commander |  |  |
| 542 | `go` | Commander |  | A Move, unless the tank was already sent there lately. |
| 581 | `rotation_points` | Commander | yes | A place the world lately refused a drive to, and no sea trip on the way there. |
| 589 | `rotate` | Commander |  | Idle: rotate between the defense point (the base's post) and the push point instead of waiting (rotation.rs). |
| 614 | `wing_rotate` | Commander |  | A wingman in formation by a parked leader (`WING_PARKED`): rotate between its slot (the defense point) and a point toward the objective, inside the leader's gathering ring (rotation.rs). |
| 653 | `over_water` | Commander |  |  |
| 660 | `home` | Commander |  | The nearest depot of my side, for resupply: of those the world has not lately refused as over the water, if any; else the nearest, and the drive's refusal sends the tank by sea. |
| 668 | `set_doing` | Commander |  | Low on what the goal needs: go home, and stay until full. |
| 675 | `why_text` | Commander | yes | A rung's reason in words, for the coordination screen: the code of `why` put into the sentence it has always been. |
| 704 | `needs_home` | Commander |  |  |
| 722 | `keep_inner` | Commander |  | What the inner doctrine may still do under a goal: drones always; shots at tanks unless the goal wants the gun for itself. |
| 732 | `hunt` | Commander |  | `area`: only pillboxes within this distance of this point (a wingman's, by its leader); None for the whole map. |
| 869 | `fly_wing` | Commander |  | The wingman's order of things, from Paul's notes: protect the leader; hit what is ahead of it; take the pillbox it is taking, or another near it; take a depot near it; help another ally's fight; else close up behind it. |
| 1045 | `alliances` | Commander |  | A bot seat's sides: answer what is put to it (yes to the leader and its side, yes to anyone who has not shot at it); ask to join the leader's side while not on it; and leave the side, keeping what it took, only once e... |
| 1081 | `fortify` | Commander |  | Build the castle: guard the point as `defend` does, and keep the squad on the plan whenever it is aboard and nothing threatens. |
| 1143 | `base_of` | Commander |  | The base's centre: the castle's, else the depot of my side nearest the tank when first asked, else where the tank is. |
| 1180 | `post` | Commander |  | Where the tank waits at the base: behind the gate when there is a castle, else beside the centre. |
| 1190 | `base_threat` | Commander |  | The threat to the base, if any: the hostile tank nearest it within BASE_ALARM, and the alarm a pillbox of the base shot at raises (by a gun it may not see: indirect fire, a longshot). |
| 1216 | `neediest` | Commander |  | The base's pillbox that most wants the squad: one dead first (ours to mend, theirs to take back), then the most hurt of ours. |
| 1228 | `defend_base` | Commander |  | Defend the base: out to meet the threat as far as BASE_ALARM, else on guard at the post; the squad mends or takes back the neediest pillbox of the base whenever no hostile tank is in the leash. |
| 1255 | `outlying` | Commander |  | An outlying pillbox of mine, nearest the base: one to bring in. |
| 1267 | `turrets` | Commander |  | Turrets: fetch an outlying pillbox of mine and set it down in the next bastion. |
| 1354 | `auto` | Commander |  | The ladder: each rung's condition, top down; the first that holds (and is on) is done this tick. |
| 1476 | `slot_point` | Commander |  | My place in the leader's wedge (`WEDGE`), where it will be in a second at the leader's speed. |
| 1498 | `bridge_near` | Commander |  | A bridge under the leader, on its next three route tiles, or on the line behind it within `back` tiles (its file still on it). |
| 1515 | `helpers` | Commander |  | A fleet leader's wingmen near enough to lay its bridge. |
| 1528 | `bridge_jobs` | Commander |  | A fleet's leader, about to travel: half its wingmen within `GATHER_R`, or it holds for them (`GATHER_FOR` at most). |
| 1564 | `duck` | Commander |  | The duck (roadmap 12d, LR 19): a hostile tank with shells in direct range and line, its gun ready within `duck_ready` and before mine, and my armour share at or under `w_duck`: where to go, out of its range or line, a... |
| 1613 | `gathering` | Commander |  |  |
| 1648 | `expand` | Commander |  | Expand: the nearest depot not of my side to the base, its guards first; with none to be had, any pillbox on the map. |
| 1690 | `flush_say` | Commander |  | Team chat, oldest first, one line a second; a backlog of more than three is stale and dropped. |
| 1707 | `revive` | Commander |  | Revive (Paul, 16 October: "can we have an ally repair a fallen tank"): an allied tank down, not yet dead, inside `REVIVE_REACH`, with time left for the drive, the squad's walk and the work. |
| 1804 | `help_chain` | Commander |  | Chain help (Paul: "tell the bots to prioritize chain help"). |
| 1877 | `chain_target` | Commander |  |  |
| 1885 | `flank` | Commander |  | A seat's flank in a group attack: seats taken three ways, left, centre and right, each coming in sixty degrees round from the way it was coming, so three bots answering one ping close from three sides. |
| 1891 | `flank_word` | Commander |  |  |
| 1897 | `called` | Commander |  | What a ping marks, in a word or two: the hostile tank by it, or the place. |
| 1905 | `hear_pings` | Commander |  | A teammate's ping within `RALLY_REACH`, if this bot answers them. |
| 1955 | `call_enemy` | Commander |  | A hostile tank in sight that nobody of the side has called: ping it, and go for it too. |
| 1987 | `rallying` | Commander |  | Answering a ping: to it, and onto any hostile tank near it, with the doctrine's gun free. |
| 2091 | `fire_word` | Commander |  | The target the doctrine's gun is on and why, for `status`. |
| 2100 | `blocked` | Commander |  | No line from here: round the target to the next firing point; a whole circle and a half without one, and it is given up. |
| 2112 | `defend` | Commander |  | Hold by a point, fight what comes, mend what is ours there. |
| 2116 | `defend_within` | Commander |  |  |
| 2188 | `name` | Commander |  |  |
| 2192 | `settings` | Commander |  |  |
| 2196 | `set_settings` | Commander |  |  |
| 2200 | `rung` | Commander |  |  |
| 2204 | `beach` | Commander |  | The inner RuleBot's: the doctrine runs the landfall. |
| 2207 | `dead` | Commander |  |  |
| 2212 | `act` | Commander |  |  |
| 2216 | `status` | Commander |  |  |
| 2225 | `think` | Commander |  |  |
| 2229 | `think_on` | Commander |  |  |
| 2236 | `think_map` | Commander |  |  |
| 3102 | `bastions` |  |  | The settings with the castle of 11b and 11h: three towers and two bastions, no cap. |
| 3110 | `one_pillbox` |  |  | Two seats, no swarm, no time limit, no pillboxes but one placed `away` tiles from seat 0 toward the middle, owned by `owner`. |
| 3128 | `run` |  |  | Run seat 0 under `goal` for `seconds`; the other seat sits still. |
| 3193 | `a_pair` |  |  | Three seats: 0 the leader, 1 its wingman on the same side, 2 the enemy; the wingman starts beside the leader, nobody else moves. |
| 3211 | `fly` |  |  |  |
| 3431 | `free_near` |  |  | A free tile about `r` from `from`: open ground, off depots and pillboxes' spacing. |

Tests (23):

- `a_capped_castle_fills_then_lines_the_road` (line 2734): Paul's castle in play (roadmap 12h): alone, the capped castle goes up with two pillboxes inside, the first outlying pillbox is carried to the slot on its road, and the rest to posts on the road toward the side's secon...
- `a_far_fetch_gathers_the_fleet` (line 2784): The escort (roadmap 12f): with `escort` on, a leader whose fetch trip runs past `fetch_gather` waits at the base for its wingman before it goes, and the pillbox still comes home.
- `a_nan_does_not_panic_the_brain` (line 2833): Backlog 19 (LR 23): a NaN anywhere in what a tank sees used to panic the brain at the first `partial_cmp().unwrap()` that met it.
- `a_tank_ducks_a_gun_ready_before_its_own` (line 2865): The duck (roadmap 12d, LR 19): a hostile gun in range and line, ready before mine, is ducked out of range; not when my gun is ready first, not with `w_duck` at 0, not with the switch off.
- `a_teammate_finishes_a_part_laid_bridge` (line 2949): A part-laid bridge on the team board (roadmap 12f, LR 6.2): a teammate on Auto with material, nothing to do but guard, takes it and lays the rest, and it comes off the board.
- `the_posture_leans_the_ladder` (line 3008): The posture leans the ladder (LR 24): with no posture or a weight of 0 the pick is the first rung that holds, for every one of the 4096 combinations; at 0.25 a leaned rung climbs one rung, at the default 0.5 every run...
- `a_fleet_crosses_a_bridge_in_file` (line 3045): On a bridge a wingman's slot is in file straight behind the leader (roadmap 12f3); off it, the wedge; without the capability, the wedge on the bridge too.
- `a_switch_comes_back_and_a_slider_keeps_the_target` (line 3079): The sliders (backlog 15): a switch off and on again mid-match gives the role's own setting back, and a weight moved keeps the turret's target.
- `a_commander_takes_a_pillbox` (line 3162): 11a's proof: a live pillbox of the enemy's, 25 tiles off, is shot dead from outside its ring and taken by the squad inside ninety seconds, and it never lands a shell on the tank.
- `a_commander_walks_round_a_wall_to_a_line` (line 3173): A wall on the near side of the pillbox: the first firing point has no line, so the hunter walks round to one that has.
- `a_wingman_covers_its_leader` (line 3223): The wingman's first rule: a hostile tank that comes up to the leader is hit, and the wingman never leaves the leash to do it.
- `a_wingman_takes_the_pillbox_by_its_leader` (line 3272): The pillbox by the leader is the wingman's to take; one far from the leader is not.
- `a_wingman_leaves_only_at_one_hundred_percent` (line 3309): The defection gate: with one pillbox not the side's, never; with all of them and every depot, after `TOTAL_HOLD` and not before.
- `a_commander_fortifies_its_depot` (line 3345): 11b's proof: alone on the island for five minutes, the squad builds the castle round the home depot: the towers and the rear pillbox, most of the wall, the gate left open, and a route from the front into the depot com...
- `a_commander_defends_its_base` (line 3442): Defend the base: a tank parked 12 tiles out is a threat (beyond DefendDepot's leash, inside the base's alarm); it is met under the base's guns and shot, and the hurt pillbox is mended once it is gone.
- `a_commander_scoots_from_unseen_fire` (line 3487): Shelled from twenty tiles, beyond sight: the tank scoots off the spot inside a second, puts a recon up, and so sees the shooter.
- `a_bot_answers_a_chain` (line 3547): Chain help: a person who chains a pillbox alone is answered by the nearest bot of the side, which hitches on and hauls beside them, so the pillbox moves when the person drives off.
- `a_bot_revives_a_downed_ally` (line 3593): Revive: a bot whose ally goes down twelve tiles off drives over, sends its squad and has the ally back at half armour inside the twenty seconds.
- `two_crews_haul_a_pillbox` (line 3634): Haul: two crews given the same job chain an ally's pillbox and drag it to the spot, then let go.
- `a_commander_climbs_the_ladder` (line 3690): Auto, alone: fortify first; once the castle stands and the base has been quiet, the two outlying pillboxes of ours are lifted and set down in the turrets; a tank at the base sends it to defend.
- `the_ladder_in_a_match` (line 3773): The window's solo match, headless: the map in LADDER_MAP, seat 0 on Auto against three RuleBots seeded as the window seeds them.
- `a_commander_takes_a_neutral_pillbox_without_a_shot` (line 3816): A neutral pillbox is not shot at all: the squad takes it live.
- `a_commander_defends_its_depot` (line 3830): Defend depot: an intruder that drives up to the depot is engaged, and the defender never strays past the leash.

## crew.rs

The fleet commander and its crews (Paul, 16 October: "each side has a fleet commander and they request and organize fort construction").  One Commander a side commands: the leader of a bots' team, the builder of a person's. It keeps the side's castle plan (fortify.rs) as it always did, building with its own squad; what is new is that it does not build alone. It talks to the side through the world (`Command::Request`, `Event::Requested`), so it works the same over a network and every window of the side reads it on the newswire:  1. Muster, every `MUSTER_EVERY` while the plan is unfinished: "the fort is at (x, y); crews report". 2. A crew free for work within `CREW_REACH` of the fort answers Ready. 3. The commander gives each ready crew the next item of the plan nobody has (a wall, the road, a tower), or, for a crew that owns an outlying pillbox, a turret slot to bring it to (Fetch: lift it, carry it, set it down). 4. The crew does it: fells forest first when its squad is short, drives in reach, sends its squad, and when the world answers (built, refused, abandoned) reports Ready again for the next. 5. An item not done `JOB_FOR` after it was given out is given out again; one that has failed `FAILS` times the plan does without.  Who crews: rangers and a bots' team's wingmen, and a person's wingmen only while the person is at the fort, so they still go where the person goes. Guards guard.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 45 | `job_for` |  |  |  |
| 57 | `job_tile` |  |  |  |
| 78 | `crews` | FleetCommand | yes |  |
| 82 | `failed` | FleetCommand |  |  |
| 100 | `think` | FleetCommand | yes | Every tick: the requests to send. |
| 221 | `reset` | Crew | yes | A new tank: the job it had is someone else's now (the commander gives it out again when it is late). |
| 227 | `muster_at` | Crew | yes |  |
| 232 | `hauling` | Crew | yes | What this crew is hauling, if its job is a haul. |
| 239 | `working` | Crew | yes |  |
| 244 | `laying_road` | Crew | yes | The job is a road or a bridge tile: work wherever the fleet is. |
| 248 | `ready` | Crew |  |  |
| 255 | `finish` | Crew |  |  |
| 264 | `walk` | Crew |  |  |
| 279 | `adopt` | Crew | yes | Every tick, before anything else of the bot's can return: the jobs given it. |
| 289 | `take_jobs` | Crew | yes |  |
| 315 | `think` | Crew | yes | Every tick. |

## cycle.rs

The base cycle (roadmap step 12f1, LAB-REPORT-BRAIN.md section 6.1; Paul, 18 October: "after they fill up on base resources, add to the base 1 item, 1 improvement, and attempt to build roads to other established bases or pillboxes"). A tank that has just filled up at the depot (shells, armour, and the squad's material, which a depot restocks) gives the base something before it goes back out:  1. one item of the castle plan (a wall, a tower, the road), felling first if the squad is short; 2. one improvement: the most hurt pillbox of the base mended; 3. road toward the nearest established friendly pillbox or depot not yet joined to the base, `connect_tiles` tiles a visit, so the roads grow out from the base over the match and become the way a hauled pillbox comes home (crew.rs `Haul`).  Each stage gets `STAGE_FOR` and is then passed over, so the cycle never holds a tank long; a hostile tank near ends it (the Commander checks that before asking).

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 62 | `watch` | Cycle | yes | Every tick: a fill just ended starts the cycle. |
| 69 | `begin` | Cycle | yes |  |
| 76 | `active` | Cycle | yes |  |
| 80 | `advance` | Cycle |  |  |
| 93 | `road_to` | Cycle | yes | The road tiles to lay from the base toward `to`: the straight line's grass and rubble past the yard, at most `n`. |
| 118 | `improvement` | Cycle | yes | The improvement the base most wants, by weight (`w_mend`, `w_mine`, `w_works`): the most hurt pillbox of the base mended (worth its hurt); a mine on the approach, past the wall facing the threat and off the road (wort... |
| 152 | `junction` | Cycle | yes | A road junction: a road or bridge tile with three or more road or bridge tiles beside it. |
| 164 | `road_posts` | Cycle | yes | Road posts (roadmap 12h; Paul: "they need to not stick at junctions but connect bases and castles"): on the line from the base to each of `to` (other bases and castles of the side), nearest first, a point every `spaci... |
| 197 | `think` | Cycle | yes | One think. |

Tests (4):

- `a_full_tank_gives_the_base_an_item` (line 318): One seat, its squad full: the cycle gives one plan item, then (no hurt pillbox) passes the improvement, then lays road toward the next friendly pillbox, a tile a trip.
- `the_improvement_goes_by_its_weights` (line 371): The improvement by weight: a hurt pillbox is mended before a mine at the defaults; with `w_mend` at 0 the mine; with both at 0, nothing.
- `posts_line_the_road_and_keep_off_junctions` (line 391): Posts on the line between two bases, beside the road, every spacing, none by a junction or by a pillbox already there.
- `the_road_runs_past_the_yard_and_stops_at_water` (line 423)

## facts.rs

What the map says, worked out once (roadmap steps 12b and 12f2, LAB-REPORT-BRAIN.md sections 5 and 6.2). The islands: every tile that is not water labelled with its land mass, each mass with its area and its forest; refreshed every `REFRESH`, since trees fall and bridges join masses. The crossings: `Map::crossing` is a Dijkstra over 65,536 states, and a failed one (a gap over `BRIDGE_MAX`, or no way) was asked again four times a second while the bot waited at the shore; here each answer, failures too, is kept for `CROSSING_FOR`.  And the prices that use them: a base site on a small island with few trees scores low, and a crossing is bridged or sailed by what each costs in material, a bridge past `bridge_soft` tiles costing the square of its extra length, every tree dearer where trees are scarce.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 42 | `refresh` | MapFacts | yes | Label the islands if they are older than `REFRESH`. |
| 82 | `island_at` | MapFacts | yes | The island under a point, if it is land and labelled. |
| 91 | `islands` | MapFacts | yes |  |
| 97 | `crossing` | MapFacts | yes | `Map::crossing`, its answers kept for `CROSSING_FOR`, the failures with them. |
| 111 | `forget_crossings` | MapFacts | yes | Forget the crossings: the map changed under them (a bridge laid). |
| 117 | `scarcity` | MapFacts | yes | Trees are scarce here: what a base will ask for over what the island holds (LR 6.2's s). |
| 123 | `site_score` | MapFacts | yes | A base site's worth (LR 6.2): enough trees, less a small island. |
| 133 | `crossing_prices` |  | yes | What a crossing of `len` water tiles costs as a bridge and as a boat, with trees `scarcity` dear (LR 6.2). |
| 143 | `bridge_it` |  | yes | Bridge it (true) or sail it (false). |
| 152 | `island_map` |  |  |  |

Tests (2):

- `the_islands_are_counted` (line 176)
- `a_small_island_takes_a_boat` (line 191)

## fleet.rs

Fleets (Paul, 15 October: "can we start a game with ... 3 teams of base building, 5 on each, and 1 random aggro bot"): sixteen seats dealt into three teams of five neighbours round the map's edge and one RuleBot alone, every team's bots Commanders that share their leader's base.  The starts run north, east, south, west, the corners, then the eight between (`Map::generate`); a team is five starts side by side, so its base is in reach of all five:  ```text team 0: NW, N-west, N (seat 0, the leader), N-east, NE team 1: E-north, E (seat 1, the leader), E-south, SE, S-east team 2: S (seat 2, the leader), S-west, SW, W-south, W team 3: W-north alone, the aggro RuleBot ```  A bot team's leader is on Auto and builds the castle at its own depot; the others take its depot for their base too: a guard on defend base, and a fleet of three wingmen (two of them called rangers, crews for the castle) in a wedge on the leader. Seat 0's team assists the person in it: a builder on Auto whose castle goes round the person's depot, a guard on it, two wingmen on the person.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 55 | `word` | Role | yes |  |
| 68 | `teams` |  | yes | The team of every seat. |
| 74 | `role` |  | yes | A seat's role and its team's leader. |
| 82 | `role_by_rank` |  | yes | A seat's role by its place in its team, the leader first. |
| 103 | `team_role` |  | yes | Any team, as the window deals one (`--teams N`, seat i on team i % N; the default match): `team` is its seats in order, the first the leader. |
| 110 | `team_brain` |  | yes | The brain for a seat of a team dealt by the window, named for its role. |
| 136 | `set_settings` |  | yes | The process's weights for every Commander made from now on. |
| 143 | `settings` |  | yes | The weights a new Commander gets here. |
| 148 | `with_settings` |  | yes | Run `f` with this thread's Commanders made with `s`. |
| 156 | `new_match` |  | yes | A new match is being dealt: each team gets a new board. |
| 161 | `board_for` |  | yes | The board of `leader`'s team in the match being dealt. |
| 175 | `equip` |  | yes | A Commander of `leader`'s team: the weights, and the team's board if the board is on. |
| 189 | `fleet_of` |  | yes | The fleet (Paul, 17 October: "i want to see the tanks prioritise grouping and traveling in formation. |
| 200 | `name` |  | yes | The seat's name in the roster: team and role. |
| 208 | `seat_zero_team` |  |  |  |
| 214 | `brain` |  | yes | The brain for a bot seat. |
| 227 | `brain_as` |  | yes | A role's brain. |
| 239 | `commander_as` |  | yes | A role's Commander, to change before it plays (a test's tactics); None for the aggro seat, a RuleBot. |
| 746 | `rung_line` |  |  | Seat-seconds a rung, in the ladder's order, for a census line. |
| 753 | `killed_by` |  |  | What took a tank, read from the events of the tick it died in (backlog 28): the weapon whose hit names it, whether it sank in a boat, and whether the shooter was the lone RuleBot. |

Tests (8):

- `three_teams_of_five_and_one_alone` (line 278)
- `a_fleet_match_plays_out` (line 297): A fleet match with no time limit is decided: one side is left inside half an hour.
- `tactics_pay` (line 327): The window's default match: the naval map, sixteen seats dealt into three teams (seat i on team i % 3), every bot in a team role and seat 0 on Auto as a person on the autopilot would be.
- `kiting_pays` (line 384): Does kiting pay (tactics.rs `kite`)? As `tactics_pay`, every team with the weights, one of the three kiting and two not, the kiting team turned round the seats over the seeds.
- `the_default_match_plays_as_teams` (line 440)
- `quad_census` (line 519): The quad match headless (scripts/quad.sh): fifteen seats, three teams, each led by a window's autopilot (a fleet commander on Auto), respawns without limit.
- `brain_census` (line 797): The brain census (roadmap 12a and 12g): sixteen seats dealt as `--fleets` deals them, one team on the trial settings and the other two on the baseline, the trial team turned round the three over the seeds so no start...
- `three_fleets_build` (line 1121): Sixteen seats as the window deals them with `--fleets`, seat 0 on Auto as a person under the coordination screen would be.

## fortify.rs

Fortification to a plan (roadmap step 11b): a castle's gate.  Round a point (a depot of the side, by default), facing the threat:  ```text threat ^ # # #   gate   # # #         # the curtain wall, WALL_R out, # #        T   =   T     # #       WALL_ARC either side of the #                =             #     threat; the gate left open =                 T the guard towers, TOWER_R out, D                   TOWER_ANGLE either side of it = the road, home through the gate P                 P the rear pillbox, REAR_R behind ```  Every pillbox is inside every other's 8-tile reach, so a tank that comes for one is under the guns of two; the gate is the only way in from the front, so the router brings an attacker through it, between the towers. A plan is a list of squad orders given one at a time. The Observation carries no map, so the plan learns the ground from the world's answers: built, refused and why, abandoned and why.  Turrets (step 11h) come later, when the castle stands and nothing has come at it for a while: `add_turrets` puts a bastion at each end of the curtain wall, set into it with one more wall tile beyond, for a pillbox carried in from the side's outlying ones. The ends are where an attacker goes round the wall instead of through the gate; each bastion is in reach of a tower and of the rear pillbox.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 134 | `castle` | Plan | yes | The castle round `centre`, gate toward `threat`, in building order: the towers and the rear pillbox (the guns first: a wall nobody covers only slows an attacker), the wall from its ends in to the gate, the road. |
| 203 | `castle_capped` | Plan | yes | A capped castle (Paul, 18 October: "we also stop filling the castle once it has ... 2 on the inside, 1 on the outside along the road"): `inside` pillboxes inside the wall (the gate's two towers, then the rear, then ei... |
| 229 | `add_posts` | Plan | yes | Road posts (roadmap 12h): slots for carried pillboxes along the roads between bases, as lifted items after everything else. |
| 240 | `add_turrets` | Plan | yes | The turrets' slots and their walls, once. |
| 327 | `turret_preview` | Plan | yes | With the turrets added (whether or not they are yet): a turret slot to fill, and yard walls or roads left. |
| 341 | `yard_left` | Plan | yes | Walls and roads of the turrets' part still to build: work for the squad while there is no pillbox to fetch. |
| 347 | `fit_post` | Plan | yes | The post on land: the nearest passable ground, not water, within 3 tiles of the centre, to the post as drawn. |
| 369 | `castle_stands` | Plan | yes | The castle without its turrets is finished: nothing left to build or wait on. |
| 374 | `turrets_stand` | Plan | yes | The turrets and their walls are all up (or cannot be). |
| 380 | `turret_slot` | Plan | yes | The next turret slot waiting for a pillbox, and whether it is forest to be felled first. |
| 385 | `felling_slot` | Plan | yes | A forest turret slot sent for felling: pending again after. |
| 392 | `turrets` | Plan | yes |  |
| 397 | `ordered` | Plan | yes | A turret slot the squad was sent to set a pillbox down on. |
| 405 | `free_item` | Plan | yes | The first item a crew could take: pending, not a turret, not in `taken` (given out already, or the commander's own squad's). |
| 410 | `free_slots` | Plan | yes | Turret slots waiting for a pillbox, not in `taken`. |
| 415 | `own_order` | Plan | yes | The commander's own squad is on this tile: crews keep off it. |
| 420 | `give_up` | Plan | yes | A crew failed here twice: the plan does without it. |
| 427 | `gate` | Plan | yes | The gate's middle: where the road crosses the wall. |
| 433 | `castle_left` | Plan | yes | The share of the castle (gate, walls, towers) not yet built or given up: 1 just planned, 0 standing (the posture's Fortify, LR 25). |
| 443 | `slots_left` | Plan | yes | The share of pillbox slots (turrets, the road slot, road posts) still waiting for a pillbox: the posture's Connect (LR 25). |
| 451 | `done` | Plan | yes |  |
| 455 | `item_at` | Plan |  |  |
| 460 | `hear` | Plan | yes | Read the world's answers to the plan's orders. |
| 534 | `next` | Plan | yes | The next order for an idle squad with `material`, or why none. |
| 585 | `progress` | Plan | yes | What the plan has, for the status line. |

Tests (2):

- `the_capped_castle_has_two_inside_one_on_the_road` (line 609): The shape, on paper: towers inside the wall and either side of the gate, every pillbox in every other's reach, the gate open, the wall on the threat's side only.
- `the_castle_is_drawn_right` (line 630)

## hop.rs

Island hopping: a bot that has taken everything it can reach goes for what it cannot, across the water.  The first naval match had a tank sit on its island for the rest of the match once the pillbox there was taken: every depot and pillbox left was over the sea, the world refused each drive to one as unreachable, and the bot waited for something to come to it. This is what it does instead, shared by the RuleBot and the Commander:  1. Idle for `HOP_IDLE` seconds (standing, the squad aboard, nothing hostile in sight) with something untaken on the map: a depot not of its side, or a pillbox or structure that is nobody's or dead. The nearest becomes the target. 2. Drive to it. The world routes by land; if it says the target is unreachable, it is over water. 3. Over water: board a moored boat within `BOAT_WALK` if there is one (`BOAT_NEAR` with a launch in hand); else drive to the bank of the launch site (the observation's water beside the bank) nearest the way to the target and launch from it (`Command::Embark`, which needs no material). If the bank is refused or not reached in `BANK_FOR`, the squad builds a boat there, for five material: short of it, restock at a depot or fell forest first. 4. Afloat, sail for the target itself: the world routes over the water and up the nearest beach to it. 5. Ashore again, the doctrine takes over within `ARRIVED` of the target; still unreachable from the new shore, back to 3. A target that took `MAX_BOATS` boats, or that even the sea does not reach, is given up.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 165 | `reset` | Hop | yes | A new tank at the start: whatever it was doing is over; what it gave up on stays given up. |
| 177 | `say` | Hop |  | Say what it is doing: the code and the words (status.rs). |
| 183 | `adopted` | Hop | yes | Laying a teammate's bridge taken from the board. |
| 187 | `active` | Hop | yes |  |
| 193 | `bridge_front` | Hop | yes | The next tile of the bridge, or of the road on from it, being laid: the fleet's job (commander.rs). |
| 199 | `bridge_job` | Hop | yes | The bridge being laid, for the team board: its water tiles still to lay, in order, and what it crosses to. |
| 207 | `bridge_refused` | Hop | yes | A bridge to `target` given up here lately: not one to take. |
| 214 | `adopt_bridge` | Hop | yes | Take a teammate's part-laid bridge from the board: lay the rest of `tiles` toward `target`, and stop there. |
| 231 | `launching` | Hop | yes | Ashore, the target over the water: about to launch. |
| 240 | `send` | Hop | yes | An errand over the water (Paul's quad, 16 October: a guard whose post was across the sea drove at it for the whole match; a tank sent home to resupply sat on the wrong island at 25 armour): the world refused the drive... |
| 253 | `given_up` | Hop | yes | Given up on lately (`TRIED_FOR`). |
| 258 | `set_forget` | Hop | yes | How long what it gave up on stays given up (memory.rs). |
| 265 | `sent` | Hop | yes |  |
| 269 | `give_up` | Hop |  |  |
| 288 | `bank` | Hop |  | The land beside a launch site, on the tank's side of it. |
| 295 | `bridging` | Hop |  | One think of the bridge: the next water tile laid from the end so far, the tank at that end, the material felled or fetched. |
| 399 | `road_line` | Hop |  | The road on from a bridge to the target: the grass and rubble tiles on the straight line between, `ROAD_ON` at most, the last few short of the target left for it. |
| 426 | `lay_road` | Hop |  | Lay the road on, a tile ahead of the tank each time; true while it does. |
| 477 | `landing` | Hop |  | The beach to land on for `target`, if one in sight is within `LANDING_NEAR` of it: distance to the target plus its danger, the choice kept unless another is clearly better (`landing_at`). |
| 512 | `covered` | Hop |  | What covers the water here, if anything (`COVER_R`). |
| 527 | `pick` | Hop |  | Something untaken that is not where the tank is: the nearest. |
| 535 | `taken` | Hop |  |  |
| 542 | `hear` | Hop | yes | Every tick, think or not: the world's answer to an order comes the tick after it, which is rarely a think tick. |
| 595 | `think` | Hop | yes | Every think. |

## intent.rs

Holding a decision (roadmap step 12d, LAB-REPORT-BRAIN.md section 7). A tank keeps doing its last command until a new one has been confirmed over several thinks; only the survival reflexes, dodging a shell and going home to resupply, break in at once and at full throttle. Two pieces:  - `Desire`, a smoothed goal: D = (1 - alpha) D + alpha d. A goal that moves a little each think (a wedge slot ahead of a moving leader, a rally on a moving enemy) is followed smoothly instead of re-routed every tick; a jump past `jitter` tiles is taken at once. - `Confirm`, a Schmitt trigger on a choice: a challenger replaces the held choice only when it beats it by `margin` for `need` thinks in a row, and the held choice's score carries a commitment bonus.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 26 | `toward` | Desire | yes | Where to go this think, given where the raw pull says; `alpha` the share of the new pull, `jitter` the jump taken at once. |
| 36 | `reset` | Desire | yes | Forget: the next pull is taken as it is (a dodge, a new goal). |
| 40 | `at` | Desire | yes |  |
| 53 | `default` | Confirm |  |  |
| 62 | `choose` | Confirm | yes | This think's best `best` (score `best_s`) against the held choice (score `held_s`, None if it is no longer possible): what to do. |
| 95 | `force` | Confirm | yes | Take `x` now, whatever the score: a survival reflex, or an order. |
| 100 | `held` | Confirm | yes |  |
| 104 | `clear` | Confirm | yes |  |
| 125 | `update` | Advance | yes |  |

Tests (3):

- `a_challenger_needs_two_thinks` (line 142)
- `a_dodge_skips_confirmation` (line 161)
- `the_advance_holds_between_its_bands` (line 173)

## ledger.rs

The ledger (roadmap step 12g, LAB-REPORT-BRAIN.md section 8). Every census match writes one JSON line per seat: the genome (the weights, the capabilities, the seed, the role) beside what the seat did, so a high score can be played again (the sim is deterministic: the line and the map are the match) and its weights bred from (`scripts/tune.sh`).  `Tally` counts as the match runs, from the world's events and, once a second, from what each team holds; `records` writes the lines. Ratings are Elo over the ledger's matches: a genome's team placed above another's in a match counts as a win against it, so a lucky score against weak opponents counts for less than a steady one against strong ones.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 32 | `of` | Genome | yes |  |
| 88 | `default` | ScoreWeights |  |  |
| 98 | `from_long_game` | ScoreWeights | yes | The score between elimination and holding ground (`long_game`, roadmap 12h): 0 weighs the win and kills half as much again and the holdings not at all; 1 the win and kills half as much and the holdings twice; 0.5 is t... |
| 105 | `score` |  | yes |  |
| 117 | `new` | Tally | yes |  |
| 122 | `hear` | Tally | yes | After each `step`: the tick's events; once a second, the holdings. |
| 172 | `records` | Tally | yes | The match is over (or out of time): a line per seat with a genome. |
| 189 | `write` |  | yes | Append records to `dir/ledger.jsonl`. |
| 200 | `read` |  | yes | Read a ledger; bad lines are skipped. |
| 207 | `mean_se` |  | yes | The mean of a sample and its standard error (the sample's standard deviation over the root of its size); the error is 0 for fewer than two. |
| 223 | `ratings` |  | yes | Elo ratings by genome over the ledger's matches: within a match, each genome's best seat score against every other genome's, a win for the higher. |

Tests (3):

- `ledger_ratings` (line 275): The hall of fame: the ledger's genomes by Elo, best first, with the weights each changed (`LEDGER`, default reports/ledger/ledger.jsonl).
- `the_standard_error_of_a_sample` (line 287)
- `a_ledger_line_round_trips_and_rates` (line 297)

## lib.rs

treadcore-brain -- the controller interface, and one controller.  The prior art called them Brains: a program that takes the tank's place at the controls, given only what the tank can see. Same idea here. A Brain gets an `Observation` -- the player's view of the world, filtered by the host, nothing a human client does not also get -- and returns `Command`s, which the world validates like anyone else's. A Brain cannot cheat because it has no more access than a keyboard.  Three implementations: `RuleBot`, which fills empty seats and is good enough to kill a careless player; `Commander` (`commander.rs`), which drives the player's own seat toward a goal the player chose; and `JsonBrain`, which speaks the observation and commands as JSON over stdin/stdout so an external program -- a Python script, an LLM harness -- can play without linking this crate.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 42 | `name` |  |  |  |
| 43 | `think` |  |  |  |
| 47 | `think_on` |  |  | `think` with the map (the host's world map, or a window's copy): public, as every window draws it, and what plans a bridge or a road needs (hop.rs). |
| 52 | `status` |  |  | What it is doing, in a line, for a window that shows its fleet. |
| 57 | `settings` |  |  | Its weights and capabilities, for the ledger (weights.rs); None for a brain without them. |
| 62 | `act` |  |  | What it is doing, as a code (status.rs): for logic and the census, where `status` is for people. |
| 67 | `rung` |  |  | The Auto rung this think took, if it reached the ladder (the census, LR 26); None for a brain without one. |
| 72 | `beach` |  |  | Afloat: the beach it is running for, if any (the census's read of where the gun stands, backlog 34). |
| 77 | `dead` |  |  | Each tick the seat is dead, in place of a think: what a brain counts of its fall (the dodge's meter, LR 27). |
| 80 | `set_settings` |  |  | New weights mid-match (the coordination screen's sliders, backlog 15); a brain without them ignores it. |
| 88 | `retire_taken` |  | yes | Run every bot seat for one tick. |
| 96 | `run_bots` |  | yes |  |
| 147 | `update` | Grace |  |  |
| 160 | `fair` | Grace |  | A fair target: the first minute is over, or the tank has been seen to move or fire. |
| 340 | `watch` | Dodge | yes | The meter's reading of the dodge under way: hit (or killed) between its start and its shell's landing, or clear of it. |
| 350 | `order` | Dodge | yes |  |
| 357 | `order_with` | Dodge | yes | `jink`: answer a hit with a jink too. |
| 402 | `holding` | Dodge | yes | A dodge under way: hold it while the tank drives to its point, and, by `dodge_hold`, that share of the shell's flight once there (standing out of the splash rather than back to its errand under the shell). |
| 413 | `lead` |  | yes | Where to aim a direct shot at a moving tank: where it will be when the shell lands (40 tiles a second, 0.8 tiles of hit radius; a tank at full speed moves a tile in the flight from 10 tiles off, so a shot at where it... |
| 452 | `update` | Form |  | Look again if it is time. |
| 481 | `patience` | Form | yes | The patience, scaled: bold waits less. |
| 485 | `standoff` | Form | yes | The firing position's distance, scaled: careful stands further off. |
| 489 | `home_armor` | Form | yes | The armour below which the bot goes home: careful goes sooner. |
| 523 | `hurt_by_player` | RuleBot | yes | Has this player's fire hit the bot's tank this match? |
| 527 | `new` | RuleBot | yes |  |
| 537 | `remember_guns` | RuleBot |  | Hostile tanks kept where last seen, for `landfall_memory` seconds (backlog 36): one seen now is kept where it is; a memory goes when its time is up, or when its spot is in the team's sight and the tank is not there. |
| 553 | `known_guns` | RuleBot |  | The hostile tanks the landfall prices: those seen now and those remembered (`landfall_memory`). |
| 558 | `standing_off` | RuleBot | yes | Afloat and standing off a covered beach (`landfall_wait`). |
| 568 | `landfall_order` | RuleBot |  | Afloat (SPEC TNK-08): run for land. |
| 656 | `rookie` | RuleBot | yes | The same doctrine, badly (PLAYABILITY item 9): thinks half as often, launches nothing for `ROOKIE_LATE` seconds and then keeps one interceptor up, aims where the tank is with `ROOKIE_SCATTER` tiles of error and no lea... |
| 661 | `without_pillboys` | RuleBot | yes | The same doctrine without the squad: the control group. |
| 665 | `nearest_enemy` | RuleBot |  |  |
| 678 | `works_order` | RuleBot |  | The squad's works (steps 10i and 10j), in this order: a power plant when a structure of this bot's stands dark with no plant of the side in reach (everything else of the economy is dead without one); a jammer once a s... |
| 704 | `works_need` | RuleBot |  | What this bot's structures within `reach` want built beside them, first want first: a power plant for a dark one with no plant of the side in reach; a jammer, once the side has been hit, for one no jammer covers; a wo... |
| 723 | `fell` | RuleBot |  | Scrap: the nearest forest in reach, if the squad has room for it. |
| 732 | `nearest_depot` | RuleBot |  | The nearest depot of the side (or not), of those the world has not refused a drive to as over the water. |
| 741 | `name` | RuleBot |  |  |
| 746 | `beach` | RuleBot |  | The beach `landfall_order` chose, while afloat. |
| 750 | `status` | RuleBot |  |  |
| 758 | `think` | RuleBot |  |  |
| 762 | `think_on` | RuleBot |  |  |
| 768 | `think_map` | RuleBot | yes |  |
| 1365 | `new` | Defender | yes |  |
| 1393 | `ring` | Defender |  | The ring of tiles at `r` round `c`, in the order to lay them: the far side first. |
| 1416 | `mouth_sites` | Defender |  | Candidate sites for the mouth pillbox: along the gate direction at `MOUTH`, then a little to either side, then further out. |
| 1429 | `approach_sites` | Defender |  | The approach mines: three across the gate direction at `APPROACH`, the road's tile in the middle. |
| 1436 | `name` | Defender |  |  |
| 1439 | `think` | Defender |  |  |
| 1719 | `spawn` | JsonBrain | yes |  |
| 1743 | `name` | JsonBrain |  |  |
| 1746 | `think` | JsonBrain |  |  |
| 1763 | `drop` | JsonBrain |  |  |
| 1769 | `schema_examples` |  | yes | Example documents for the JSON brain, printed by the server. |
| 2242 | `name` |  |  |  |
| 2245 | `think` |  |  |  |

Tests (17):

- `a_bot_drives_out_from_under_a_shell` (line 1801): PLAYABILITY item 9: a rookie is beatable.
- `a_dodge_holds_out_of_the_splash_by_its_weight` (line 1828): `dodge_hold` (LR 27): a tank at its dodge point before the shell lands stays there at 1, and is free to its errand at 0 (the dodge before LR 27).
- `a_shy_boat_runs_for_the_beach_clear_of_the_guns` (line 1857): `landfall_shy` (backlog 30, LR 31): of two beaches, the nearer one's run passing a hostile gun in reach, at 0 the boat takes it (its price is the beach and the heading only), at 1 the farther one clear of the guns.
- `a_boat_stands_off_a_covered_beach_for_its_wait` (line 1880): `landfall_wait` (backlog 33, LR 33): the only beach covered by a seen gun 8 tiles off, at 0 the boat lands; at 10 it backs off out of the gun's reach, and lands once the 10 s are spent.
- `a_gun_seen_and_lost_still_prices_the_run` (line 1907): `landfall_memory` (backlog 36, LR 36): a gun seen beside the near beach's run and then lost from sight still prices that run for its memory's seconds (here with `landfall_shy` 1), and not after.
- `rulebot_kiting_pays` (line 1949): RuleBots win more than twice as often, kill more and outlive them, and the rookies still kill something.
- `rookies_are_beatable` (line 1994)
- `a_rulebot_drives_to_its_zone_to_build` (line 2034): Step 10k: a RuleBot with the scrap drives to a dark refinery of its own thirty tiles off, out of the squad's reach, and powers it.
- `a_rulebot_powers_its_zone_then_builds_a_workshop` (line 2070): Step 10j: a RuleBot holding a dark refinery builds a power plant beside it first, then a workshop; no jammer, because nothing of its side has been hit.
- `a_rulebot_jams_after_a_hit` (line 2105): Step 10i: a RuleBot holding a powered zone builds no jammer until a structure of its side is hit; then it builds one on a free pad, launches nothing inside it, and drives out of it.
- `a_rulebot_answers_a_person` (line 2161): A RuleBot answers a person's proposal: yes, unless that person has hurt it.
- `pillboys_pay` (line 2195): Sixteen bots, six minutes: the rules get exercised.
- `fortification_pays` (line 2235): Roadmap step 6c's proof: a fortified depot survives a scripted attack longer than a bare one, over twenty seeds.
- `a_newcomer_gets_a_first_minute` (line 2318): PLAYABILITY.md item 1, the first minute: a newcomer who sits at the start reading the help is not shelled by the bots for the first minute.
- `defenders_hold` (line 2360): Roadmap step 6d's proof, as it came out: sixteen bots, four attacking, over ten seeds of five minutes.
- `bots_fight_to_a_result` (line 2441)
- `a_led_direct_shot_cannot_be_dodged` (line 2472): Roadmap 12d, backlog 8 (LR 19): a direct shell flies five ticks at most (10 tiles at 40 a second), and a tank at full speed turning as hard as it can moves about 0.4 tiles off the gunner's lead in that time, inside th...

## memory.rs

Memories that run out (roadmap step 12b, finding L6 of LAB-REPORT-BRAIN.md). The brain kept its given-up targets in lists that grew all match and were never looked at again: a pillbox walked round twelve times stayed given up after its guards were dead, a depot refused once over the water stayed refused after the bridge was laid. `Forget` keeps each with the tick it was put there, and a memory older than its `keep` is gone. A `keep` of `NEVER` remembers for the match, as the lists did; that is the default until the census promotes a time (the `FORGET` capability, weights.rs).

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 24 | `default` | Forget |  |  |
| 30 | `new` | Forget | yes |  |
| 34 | `live` | Forget |  |  |
| 39 | `put` | Forget | yes | Remember `x` from `now`; remembered already, the clock restarts. |
| 48 | `has` | Forget | yes | Still remembered at `now`. |
| 53 | `forget_old` | Forget | yes | Drop what has run out. |
| 60 | `clear` | Forget | yes |  |
| 64 | `remove` | Forget | yes |  |
| 69 | `len` | Forget | yes | How many are held (old ones included until the next `put`). |
| 73 | `is_empty` | Forget | yes |  |

Tests (1):

- `a_memory_runs_out_unless_kept_for_ever` (line 83)

## meter.rs

What the brain spends (roadmap step 12a, LAB-REPORT-BRAIN.md section 5, "measure first"). Counters, one set per thread, so the census tests that run side by side each count their own match: thinks, the Moves the bots asked for, the crossing searches, the plan copies, the crossing answers the cache gave instead, and the time `run_bots` took. Nothing reads them but a census and `Brain::status`-style reports; no decision depends on them, so a replay is the same with them or without.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 57 | `add` | Dodges |  | Another count added to this one. |
| 74 | `trial` |  | yes | The trial seats, a bit a player; the rest are group 0. |
| 78 | `group` |  |  | The group of the seat being run: 1 if `trial` marks it, else 0. |
| 85 | `bump` |  |  |  |
| 94 | `seat` |  | yes | A seat run, the player's: the counts after it are its group's. |
| 98 | `think` |  | yes |  |
| 101 | `moves` |  | yes |  |
| 104 | `crossing` |  | yes |  |
| 107 | `crossing_cached` |  | yes |  |
| 110 | `plan_clone` |  | yes |  |
| 113 | `posture` |  | yes |  |
| 123 | `dodge` |  | yes | A shell dodge begun, ashore or afloat, taken or dropped (LR 27). |
| 129 | `dodge_hit` |  | yes | A dodge hit before its shell landed: killed by it, and by the dodged shell itself (the hit on its landing tick). |
| 138 | `spent` |  | yes |  |
| 143 | `read` |  | yes | This thread's counts. |
| 148 | `reset` |  | yes | Start counting again from zero. |
| 154 | `add_dodges` | Meter | yes | Adds another count's dodges to this one's (a census's total). |
| 165 | `dodge_line` | Meter |  | The dodges for the meter's line, each kind that happened. |
| 184 | `micros_per_seat` | Meter | yes | Microseconds per seat-tick: what one bot costs a tick, on average. |
| 193 | `line` | Meter | yes | One line for a census: the counts and the cost. |

Tests (2):

- `the_meter_counts_and_resets` (line 225)
- `a_dodge_counts_by_its_seat_and_whether_taken` (line 242): A dodge counts under the seat that made it (the trial's or the rest), taken or dropped, ashore or afloat (backlog 31).

## rotation.rs

Rotation (Paul, 17 October: "every bot should have a push point, and a defense point. the tank does not wait around ... alternate between the adjustable push point and defense point (usually your fortress or base)"):  ```text base (defense point) ----------- push ----------- target D  <-- dwell, resupply    P  <-- dwell, look ---- out ----> <--- home ---- ```  Where a tank used to wait (on guard with nothing in the leash, the squad home and nothing to fetch, the castle standing) it rotates: out to the push point, a look round, home to the defense point, a look round, out again. A fight anywhere on the way is the caller's, as before; the rotation is only what the idle time is spent on.  The push point is `PUSH_FRAC` of the way from the base to the side's objective: the nearest depot not of the side, else the nearest pillbox not of the side, else the map's middle. A fleet shares a base, so its members work out the same point: the fleet's push point. A tank can be given its own (`own`), which is used instead.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 77 | `objective` | Rotation | yes | The side's objective seen from `base`: the nearest depot not of the side, else the nearest pillbox not of the side, else the map's middle. |
| 90 | `fleet_push` | Rotation | yes | The fleet's push point: `PUSH_FRAC` of the way to the objective. |
| 97 | `push` | Rotation | yes | This tank's push point: its own, else the fleet's (on ground it can drive to, once `fit` has seen the map). |
| 109 | `peek` | Rotation | yes | The push point as `push` would give it, without changing anything (for the window's coordination screen). |
| 121 | `wing_push` | Rotation | yes | A wingman's push point: from the parked leader toward the side's objective (or toward its own point), `WING_REACH` out at most, pulled back toward the leader onto ground (`ground`). |
| 143 | `fit` | Rotation | yes | With the map: the push point is the tile nearest it that the tank can drive to from the base, without a boat. |
| 181 | `leg` | Rotation | yes | The leg to drive: out to `push` or home to `defense`, turning at each end after `DWELL` (and at home, once refilled). |

Tests (2):

- `the_tank_rotates_between_its_points` (line 214): Out, a look, home, a look, out: the tank never stops for long.
- `a_wingman_pushes_inside_the_ring` (line 258): A wingman's push point: toward the objective, inside the leader's gathering ring, and back toward the leader off the water.

## status.rs

Status as codes (roadmap step 12b, finding L3 of LAB-REPORT-BRAIN.md, section 22). What a tank is doing was only ever a sentence, and the brain's own logic and the census read the sentence back: a guard was idle if its words began "on guard", a focus call went out if the reason said "called", the census counted "ducking" by substring. A rewording broke them silently. Now each sentence is written with a code beside it, and logic and measurement read the code; the words are for people, the code for the program.  The codes are `Copy` enums, set where the words are written (`Commander::set_doing`, `Hop::say`, the cycle's `act`). `code()` is a stable dotted name for logs, the census and the ledger.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 33 | `word` | KiteStep | yes | The words, as the status line has always had them. |
| 46 | `code` | KiteStep | yes |  |
| 59 | `stepping_in` | KiteStep | yes |  |
| 95 | `code` | HopAct | yes |  |
| 139 | `word` | CycleAct | yes | The words of an improvement (`Cycle::improvement`). |
| 151 | `code` | CycleAct | yes |  |
| 230 | `code` | Act | yes | A stable dotted name: `kite.step_in.reloading`, `hop.bridging`. |
| 241 | `plain` | Act |  |  |
| 293 | `kite` | Act | yes | The kiting step in it, if any. |

Tests (1):

- `every_code_is_its_own` (line 340)

## tactics.rs

Fleet tactics (Paul, 16 October: "we want a bunch of scalars and reasons to collaborative bot behavior. research ... fleet tactics in rts"). Three decisions a team's bots made by distance alone, each now a score of weighted considerations with the reasons kept for the status line, the log and team chat:  1. Which hostile tank to shoot (target selection). The RTS rule is focus fire: a side that puts its shots on one tank at a time takes tanks out of the fight sooner, and a tank out of the fight shoots no more (Lanchester's square law, below). So a target scores for being near, wounded, one shot from dead, afloat (a boat sinks at one hit), called by a teammate's ping, already under a teammate's gun, in range now, and threatening a teammate; and it scores less once more of us are on it than it takes (overkill: the rest pick the next). Uriarte and Ontañón score StarCraft targets the same way, by distance, threat and damage; the GA micro bots by health alone. 2. Whether to close (the engagement). Lanchester's square law: with aimed fire, a side's strength is its number squared times its quality, so three tanks beat two by more than three to two, and a side that arrives one at a time is beaten one at a time. The odds at a fight are ours squared over theirs squared; below `engage_odds` with teammates on the way a bot holds off and masses with them ("don't trickle in"), above `press_odds` it closes. 3. Who answers a call (assignment). A ping drew every bot in reach, the base's too. Now the first `answer_cap` teammates nearer to it than this bot are enough, and a crew on a job answers only inside `crew_reach`.  And where to stand (the path's end): of the flank's firing points round a target, the one least exposed to its friends' guns and to hostile pillboxes, as an influence map would pick it, without one.  And kiting (Paul, 17 October: "more directed kiting, where we estimate the range of the opponent and avoid getting hit while aggressing closer in a group"). Every tank's direct gun reaches the same 10 tiles and reloads in 4 s, and a tank's reload is in the observation (a window shows it), so the estimate is the target's reach (its direct gun while it has shells), when it can fire again, and which other guns would cover the spot a step in ends on. The bot waits on the ring just past that reach, where its own indirect gun still works (12.5 tiles); it steps in to 9 tiles, fires direct and steps out again only while the target's gun is reloading for longer than the trip takes, no other loaded gun covers the spot, and its own gun is loaded. Pressing (the square law in our favour), it also steps in while a teammate is already inside the target's reach: the target has one gun and many of us. Massing, it never does.  The considerations add; each is a weight times a value in 0..1, so a weight reads as "worth this many of the others". `Tactics::default` is the team's; a test or a bot of another temper changes the numbers.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 126 | `default` | Tactics |  |  |
| 176 | `ratio` | Odds | yes | Lanchester's square law: ours squared over theirs squared. |
| 205 | `reach` |  | yes | How far a hostile tank's gun reaches now: its direct gun while it has shells. |
| 214 | `ready_in` |  | yes | Seconds until a tank can fire again. |
| 266 | `worth` |  |  | A tank's worth in a fight: armour, and none without shells. |
| 275 | `with` | Tactician | yes |  |
| 280 | `hear` | Tactician | yes | Every tick: a teammate's ping is a call on what is under it. |
| 293 | `score` | Tactician | yes | A target's score and the reasons that made it. |
| 301 | `value` | Tactician | yes | The score alone: what `pick` asks of every candidate, with no words built for the ones that lose (roadmap 12b). |
| 305 | `rate` | Tactician |  |  |
| 358 | `pick` | Tactician | yes | The hostile tank to shoot, of those `fair` allows; the reason kept. |
| 410 | `odds` | Tactician | yes | The odds at a fight round `at`, me counted as there. |
| 434 | `stance` | Tactician | yes | Mass, engage or press, by the odds; massing ends after `mass_for` seconds whether or not the team came. |
| 489 | `kite` | Tactician | yes | Kiting `target`, coming in on `from`, at `stance`: the step in when the target's gun cannot answer it, else the waiting ring (module notes). |
| 535 | `enough_answer` | Tactician | yes | Teammates nearer a ping than me: past `answer_cap` I stay. |
| 545 | `stand` | Tactician | yes | Where to stand against `target` at `range`, coming in on `from` (a unit bearing from the target): of seven points on the arc within 60 degrees of it, the one with the fewest other guns on it, the bearing breaking ties. |

Tests (3):

- `focus_fire_picks_the_wounded_tank_a_teammate_is_on` (line 573): Two tanks with a pillbox-free field: a fresh near one, a far wounded one an ally is already shooting.
- `kiting_steps_in_only_when_the_gun_cannot_answer` (line 606): Kiting's forks, one at a time: the target loaded holds us past its reach; reloading lets us step in; a second loaded gun on the spot holds us again; our own gun not loaded by the time we would be in holds us; massing...
- `the_square_law` (line 661)

## weights.rs

Weights and capabilities (roadmap step 12c, LAB-REPORT-BRAIN.md sections 4 and 5). Every number a tuner may touch is one slot of a flat array, `Weights`, named by `W`; every part of the brain that can be switched off is one bit of `Caps`. The array is lent to whatever scores (`&Weights`), never cloned per think, and written whole into the ledger (ledger.rs) beside the match it played, so a high score can be played again and bred from.  The defaults are the constants the brain had on 17 October: the 22 weights of `Tactics` and its other settings as they were, and the new parts' numbers as the lab report starts them. A capability that is new is off by default, so the brain of 17 October is the baseline until a census promotes a part (ROADMAP goal 4).  The file form is TOML's simplest subset, a `name = value` a line and `#` comments, so a person can write one by hand and `--weights FILE` reads it without a parser crate:  ```text # bolder w_wounded = 1.5 press_odds = 1.2 caps = smooth, forget, priced_crossing ```

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 37 | `name` |  | yes |  |
| 40 | `default_value` |  | yes |  |
| 43 | `doc` |  | yes |  |
| 46 | `by_name` |  | yes |  |
| 52 | `step` |  | yes | A slider's step (the coordination screen, backlog 15): one for a rounded weight, else a tenth of the default taken to 1, 2 or 5 of its power of ten; a tenth for nothing. |
| 179 | `default` | Weights |  |  |
| 186 | `index` | Weights |  |  |
| 192 | `index_mut` | Weights |  |  |
| 211 | `doc` | Weights | yes | What a capability does, for the sliders' page. |
| 241 | `default` | Caps |  |  |
| 247 | `has` | Caps | yes |  |
| 250 | `with` | Caps | yes |  |
| 253 | `without` | Caps | yes |  |
| 257 | `all` | Caps | yes | Every capability, the new ones too. |
| 260 | `by_name` | Caps | yes |  |
| 264 | `names` | Caps | yes | The names of the bits that are on, comma-separated. |
| 277 | `fmt` | WeightsError |  |  |
| 293 | `parse` | Settings | yes | Read the file form. |
| 327 | `to_text` | Settings | yes | The file form: every weight, and the capabilities. |
| 336 | `diff` | Settings | yes | The weights that differ from the defaults: what makes a genome. |
| 342 | `nudge` | Settings | yes | A slider moved `by` steps: never under nothing, and kept to four places so a step back is the number it was. |
| 348 | `ticks` | Settings | yes | Seconds into ticks, for the time weights. |
| 355 | `from_weights` | Tactics | yes | The tactics the weights describe. |
| 405 | `eval` | Curve | yes |  |
| 425 | `utility` |  | yes | An action's utility (LR 4): its weight times the geometric mean of its considerations, raised by `commit` if it is the intent held now. |

Tests (4):

- `default_weights_are_todays_constants` (line 446)
- `a_weights_file_round_trips_and_names_its_mistakes` (line 457)
- `the_geometric_mean_compensates_and_vetoes` (line 471)
- `a_slider_steps_and_steps_back` (line 481)
