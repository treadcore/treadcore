# The brain's map

Every file of `treadcore-brain` and every function in it, with the
first sentence of its own doc comment. Written by
`scripts/brainmap.py` (`make brainmap`); do not edit by hand. The
design and its review are `LAB-REPORT-BRAIN.md`; the review of the
structure as built is `checkins/`.

23 files, 515 functions (0 with no doc comment; 39 more are a trait's, documented by it), 123 tests.

| File | Lines | Functions | Undocumented | Tests | What it is |
| --- | --- | --- | --- | --- | --- |
| [`board.rs`](#boardrs) | 811 | 28 | 0 | 8 | The team board (roadmap step 12e, LAB-REPORT-BRAIN.md sections 5 and 9). |
| [`calls.rs`](#callsrs) | 117 | 4 | 0 | 1 | Calls to friends (LR 97): a team's shared queue of requests, so a learned tank can ask its teammates for something and hear what they ask of it. |
| [`clip.rs`](#cliprs) | 270 | 12 | 0 | 1 | Clips (LR 102): a recorded run (`scenario::Recording`) drawn frame by frame into pixels, for short animated pictures of the learned tank at work. |
| [`commander.rs`](#commanderrs) | 4143 | 78 | 0 | 24 | The Commander (roadmap step 11a): a Brain for the player's own seat. |
| [`crew.rs`](#crewrs) | 584 | 18 | 0 | 1 | The fleet commander and its crews (Paul, 16 October: "each side has a fleet commander and they request and organize fort construction"). |
| [`cycle.rs`](#cyclers) | 441 | 9 | 0 | 4 | The base cycle (roadmap step 12f1, LAB-REPORT-BRAIN.md section 6.1; Paul, 18 October: "after they fill up on base resources, add to the base 1 item, 1 improvement, and attempt to build roads to other established bases... |
| [`facts.rs`](#factsrs) | 222 | 10 | 0 | 2 | What the map says, worked out once (roadmap steps 12b and 12f2, LAB-REPORT-BRAIN.md sections 5 and 6.2). |
| [`fleet.rs`](#fleetrs) | 1381 | 20 | 0 | 9 | Fleets (Paul, 15 October: "can we start a game with ... 3 teams of base building, 5 on each, and 1 random aggro bot"): sixteen seats dealt into three teams of five neighbours round the map's edge and one RuleBot alone... |
| [`fortify.rs`](#fortifyrs) | 679 | 25 | 0 | 2 | Fortification to a plan (roadmap step 11b): a castle's gate. |
| [`hop.rs`](#hoprs) | 938 | 25 | 0 | 1 | Island hopping: a bot that has taken everything it can reach goes for what it cannot, across the water. |
| [`intent.rs`](#intentrs) | 186 | 9 | 0 | 3 | Holding a decision (roadmap step 12d, LAB-REPORT-BRAIN.md section 7). |
| [`learn.rs`](#learnrs) | 1403 | 46 | 0 | 5 | Training the learned tank (LR 95, 97): the phases' training grounds and evolution strategies over a `Net`'s weights, in plain Rust on threads. |
| [`ledger.rs`](#ledgerrs) | 489 | 13 | 0 | 4 | The ledger (roadmap step 12g, LAB-REPORT-BRAIN.md section 8). |
| [`lib.rs`](#librs) | 2941 | 66 | 0 | 20 | treadcore-brain -- the controller interface, and one controller. |
| [`memory.rs`](#memoryrs) | 101 | 10 | 0 | 1 | Memories that run out (roadmap step 12b, finding L6 of LAB-REPORT-BRAIN.md). |
| [`meter.rs`](#meterrs) | 558 | 30 | 0 | 4 | What the brain spends (roadmap step 12a, LAB-REPORT-BRAIN.md section 5, "measure first"). |
| [`net.rs`](#netrs) | 593 | 21 | 0 | 7 | A learned tank (LR 95, 96): a small net from the senses (senses.rs) to an action, and the Brain that drives by it. |
| [`rotation.rs`](#rotationrs) | 301 | 7 | 0 | 3 | Rotation (Paul, 17 October: "every bot should have a push point, and a defense point. |
| [`scenario.rs`](#scenariors) | 1206 | 23 | 0 | 6 | Scenario files (roadmap backlog 24, CI-001 VII.B, LAB-REPORT-BRAIN.md section 48; Paul, 18 October: "is there a way to create description and try to attempt these via playtests faster"). |
| [`senses.rs`](#sensesrs) | 528 | 8 | 0 | 7 | The senses (LR 95, 96): what a learned tank feels, as about a hundred numbers from 0 to 1, the same on every map and every size of map. |
| [`status.rs`](#statusrs) | 357 | 9 | 0 | 1 | Status as codes (roadmap step 12b, finding L3 of LAB-REPORT-BRAIN.md, section 22). |
| [`tactics.rs`](#tacticsrs) | 694 | 16 | 0 | 3 | Fleet tactics (Paul, 16 October: "we want a bunch of scalars and reasons to collaborative bot behavior. |
| [`weights.rs`](#weightsrs) | 592 | 28 | 0 | 6 | Weights and capabilities (roadmap step 12c, LAB-REPORT-BRAIN.md sections 4 and 5). |

## board.rs

The team board (roadmap step 12e, LAB-REPORT-BRAIN.md sections 5 and 9). One per team, shared by its Commanders (`Shared`), so the team thinks on one picture of the map instead of each tank working the same things out alone from its own copy (finding L5):  - the influence map: 16 x 16 sectors of 8 x 8 tiles, ours minus theirs, each tank by its worth in a fight (armour, shells) and each pillbox by `pillbox_worth`, falling off with distance; the front is where it changes sign, territory where it is ours; - the held ground: the same of the pillboxes and depots alone, ours minus theirs; a hostile tank on it beyond the base's alarm is an incursion, which Defend reads (`w_defend_ground`, LR 68); - the defence points: the side's pillboxes and depots nearest the threat, two to four; - the members: where each is, how ready (armour times shells), what it is doing, for the coordination screen (11c's allies' goals); - the jobs the fleet commander gave out, kept, so a crew that missed the one tick of its `Requested` still finds its job; - the posture: what the team should mostly be doing; - rotation across the team: a hurt tank at the hottest point is swapped for a fresh one at a cold point, confirmed over two rebuilds (`TEAM_ROTATE`); - the part-laid bridges (`BRIDGE_JOBS`, LR 6.2): each bridge a member lays is posted with its water tiles still to lay; left part-laid (the layer gave up, went home, died), it stays a job for the next teammate with material, until laid or untouched for `bridge_keep`.  Rebuilt once a second by whichever member finds it due first; bots run in seat order every tick, so that is the same member in every replay. A Mutex because a Brain is Send + Sync; nobody waits on it.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 40 | `shared` |  | yes | A new, empty board for a team. |
| 95 | `word` | Posture | yes | The posture's name, for the screen and the census. |
| 130 | `of` | PostureScores | yes | The score of one posture. |
| 208 | `worth` |  |  | A tank's worth in a fight, for the influence: its armour share, at least a tenth; none dead or out of shells. |
| 217 | `readiness` |  | yes | A tank's readiness: armour times shells, as shares of full. |
| 227 | `sector_of` |  | yes | The sector a point is in on a 128-tile map, row-major; off the map, the nearest. |
| 232 | `sector_centre` |  | yes | The middle of a sector of a 128-tile map. |
| 237 | `sector_in` |  | yes | The sector a point is in on a grid `cols` by `rows`. |
| 244 | `centre_in` |  | yes | The middle of sector `s` of a grid `cols` across. |
| 250 | `dims` | TeamBoard | yes | Sectors across and down. |
| 255 | `sector_of` | TeamBoard | yes | The sector a point is in, on this board's grid. |
| 261 | `sector_centre` | TeamBoard | yes | The middle of a sector, on this board's grid. |
| 267 | `post` | TeamBoard | yes | Every think of a member: where it is and what it is doing, and the jobs it heard given out. |
| 293 | `post_bridge` | TeamBoard | yes | A member laying a bridge says so every think: the same bridge (a tile or the target in common) is updated, a new one posted. |
| 311 | `prune_bridges` | TeamBoard | yes | The tiles laid since, by anyone, off the jobs; a bridge laid, or untouched for `keep`, off the board. |
| 324 | `take_bridge` | TeamBoard | yes | A part-laid bridge nobody is laying, its next tile within `reach` of `at`, that `ok` takes: the nearest, now `p`'s. |
| 336 | `due` | TeamBoard | yes | A rebuild is due: never built, or `REBUILD` since the last. |
| 341 | `job_for` | TeamBoard | yes | The job given to `p` lately, if any. |
| 346 | `order_for` | TeamBoard | yes | What the board orders `p` now, if anything. |
| 351 | `done` | TeamBoard | yes | The order is carried out (home and full, or relieved): drop it. |
| 356 | `influence_at` | TeamBoard | yes | Ours minus theirs at a point's sector, as last seen. |
| 361 | `threat_at` | TeamBoard | yes | Hostile strength at a point's sector, as last seen. |
| 367 | `front` | TeamBoard | yes | The front: our sectors with a neighbour that is theirs, on the influence with the hostiles remembered (`known`), else as seen. |
| 386 | `front_dist` | TeamBoard | yes | Tiles from `p` to the nearest front sector's centre (LR 58); none with no front (no hostile seen at the last rebuild). |
| 391 | `territory` | TeamBoard | yes | The share of the map that is ours. |
| 399 | `rebuild` | TeamBoard | yes | Once a second: the maps, the points, the posture, the swaps. |
| 545 | `rotate` | TeamBoard |  | Rotation across the team (LR 9): the hottest point's least ready member against the most ready member elsewhere. |
| 584 | `placed` |  |  | Four seats, two teams (0 and 2, 1 and 3), placed; every tank seen. |

Tests (8):

- `the_team_defends_a_threat_at_the_base_not_far_off` (line 601): Defend is a threat at the base, not the map's hostile tanks summed (LR 24): eight of them 80 tiles off read 0.73 on the sector threat, over its 0.5, and sent the team home in 94% of a match's picks.
- `each_posture_scores_its_own_case` (line 623): Each posture wins on its own case (LR 25), and `posture_scored` 0 is the threshold ladder in every case.
- `a_hostile_on_the_held_ground_is_an_incursion` (line 679)
- `the_front_is_where_influence_crosses_zero` (line 708)
- `a_front_seen_is_remembered` (line 727): The front's memory (LR 58): the hostiles seen at one rebuild and out of sight at the next still make the front where they were, for `front_memory` seconds; at 0 the front goes with the sighting.
- `a_hurt_tank_swaps_out` (line 751)
- `a_part_laid_bridge_waits_for_the_next_tank` (line 771)
- `a_missed_job_is_still_on_the_board` (line 800)

## calls.rs

Calls to friends (LR 97): a team's shared queue of requests, so a learned tank can ask its teammates for something and hear what they ask of it.  A call is a kind, a place and a time. The team's learned tanks share one `Calls` (one queue a team a match, as a team board is shared, LR 58); each posts with its Signal head and feels the others' calls as senses (senses.rs: the compass's Call channel and the call contact). Every call is also said in the world as a `Command::Ping`, so scripted teammates answer it as they answer one another, and a teammate's ping heard in the world is taken into the queue as an attack call: nets and scripted brains share one language.  The brains of a match think in seat order on one thread, so the queue is read and written in the same order every time: a match is the same twice.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 57 | `new` | Calls | yes | An empty queue, for one team's match. |
| 63 | `post` | Calls | yes | Post a call; calls past their life and the oldest past `CALLS_KEPT` go. |
| 74 | `heard` | Calls | yes | The calls a tank hears now: live, and not its own; newest last. |
| 80 | `hear` | Calls | yes | Take a tick's teammates' pings from the world (`Event::Pinged`) as attack calls, once each. |

Tests (1):

- `a_call_is_heard_by_friends_for_its_life` (line 102): The queue: a call is heard by the others and not by its caller, for `CALL_LIFE` and no longer, and it keeps the newest `CALLS_KEPT`.

## clip.rs

Clips (LR 102): a recorded run (`scenario::Recording`) drawn frame by frame into pixels, for short animated pictures of the learned tank at work. Plain Rust, no image library: each frame is an RGB buffer written as a PPM, and `ffmpeg` (or anything that reads PPM) makes the GIF.  The look is the site's field (assets/site.css, night): dark grass, deep water, the learner's tank in sky blue with its armour bar, the others in the foe's red, shells in orange with their landing marked, drones as small diamonds, a hit as a white ring, and one line of text along the top in a 3 by 5 pixel font.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 29 | `hex` |  |  | A colour from its hex code, `0xRRGGBB`. |
| 35 | `new` | Image | yes | A `w` by `h` image filled with `bg`. |
| 40 | `put` | Image |  | One pixel; off the image, nothing. |
| 47 | `rect` | Image |  | A filled rectangle, its top-left at (x, y). |
| 56 | `disc` | Image |  | A filled circle of radius r round (cx, cy). |
| 67 | `ring` | Image |  | A circle's outline of radius r round (cx, cy). |
| 75 | `line` | Image |  | A straight line from (x0, y0) to (x1, y1). |
| 83 | `text` | Image | yes | One line of text, upper case, 3 by 5 glyphs scaled by `k`. |
| 99 | `ppm` | Image | yes | As a binary PPM. |
| 109 | `glyph` |  |  | A 3 by 5 glyph, each row's three bits left to right. |
| 160 | `terrain` |  |  | A tile's colour, the site's night field. |
| 177 | `render` |  | yes | One frame drawn: the recording's tiles, then what moves, seen from `learner`'s side, with `caption` along the top. |

Tests (1):

- `a_frame_draws_where_things_are` (line 260): A frame draws: the learner's tank in its colour where it stands, a hostile in red, and text in the strip; the PPM has its header.

## commander.rs

The Commander (roadmap step 11a): a Brain for the player's own seat.  A person commands the tank with a goal instead of driving it: hunt pillboxes, defend a pillbox, defend a depot, escort an ally, or hold. The Commander drives, aims and sends the squad toward the goal; a RuleBot inside it keeps the doctrine it already has for drones and for shooting at tanks, and the Commander keeps only those of its orders that serve the goal. It sees what the player sees and its orders are validated like a keyboard's, so a goal is never a cheat, and the world, the wire and the replay know nothing of it.  The pillbox, by the numbers: 60 health, 8 tiles of reach, 15 every two seconds, at tanks and drones only, never at a squad. The direct gun reaches 10 and does 45. So a live hostile pillbox dies to two direct shots from the ring between 8 and 10 tiles, taking none back; then the squad stands on it for ten seconds and it is ours. A neutral one does not have to be shot at all: the squad takes it live while the tank stays out of its ring.  Auto (step 11c's hybrid): a ladder of rungs, each with a condition, tried top down every tick; the first whose condition holds is what the tank does, and the player turns rungs on and off:  1. defend base: a hostile tank within BASE_ALARM of the base, or a pillbox of the base shot at. The tank fights it under the base's guns, a few tiles out toward it; the squad mends between shots. 2. fortify: the castle round the base is not yet standing. 3. turrets: the castle stands and nothing has come for QUIET; bring the side's outlying pillboxes home, one at a time, and set them down in the bastions at the wall's ends (step 11h). 4. hunt: a pillbox not of my side within EXPAND_R of the base. 5. expand: a depot not of my side, the nearest the base first (its guard pillboxes shot dead first, then the tank sits on it), else a pillbox anywhere. A side that only hunted pillboxes held three depots for fifteen minutes while dead teams held five each. 6. guard: on guard at the base, mending.  Low on shells or armour, the tank goes home first whatever the rung.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 131 | `label` | Rung | yes | The rung in words, for the status line and the census. |
| 143 | `index` | Rung | yes | Its place in `ALL`, top first: the index of the ladder's flags and the census's counts. |
| 152 | `leaned` | Rung | yes | The rungs a team posture leans toward (LR 24). |
| 165 | `pick` | Rung | yes | The rung to take: of those switched on that hold, the best by `(1 - w) order + w lean`, where order runs from 1 for the first rung down to 0 for the last and lean is 1 for a rung the posture leans toward. |
| 214 | `label` | Goal | yes | The goal in words, for the window's goal line. |
| 451 | `wingman` | Commander | yes | A bot seat that follows `leader` (step 11a after Paul's notes): an aggressive, greedy wingman that also runs its own alliances. |
| 460 | `new` | Commander | yes | A Commander on Manual, its doctrine a RuleBot of `seed` with tactics, the default Settings applied. |
| 473 | `set_tactics` | Commander | yes | The weights for its fights (tactics.rs); None shoots the nearest and answers every ping, as before them. |
| 479 | `set_settings` | Commander | yes | The weights and capabilities: the tactics from the weights, each part of the brain on or off by its bit (weights.rs, roadmap 12c). |
| 537 | `caps` | Commander |  | The capabilities in its Settings: which parts are on. |
| 542 | `set_goal` | Commander | yes | A new goal: what was half done for the old one is dropped. |
| 557 | `mine_or_allied` | Commander |  | Owned by this seat or an ally; an unowned thing is neither. |
| 562 | `go` | Commander |  | A Move, unless the tank was already sent there lately. |
| 604 | `rotation_points` | Commander | yes | A place the world lately refused a drive to, and no sea trip on the way there. |
| 612 | `rotate` | Commander |  | Idle: rotate between the defense point (the base's post) and the push point instead of waiting (rotation.rs). |
| 640 | `wing_rotate` | Commander |  | A wingman in formation by a parked leader (`WING_PARKED`): rotate between its slot (the defense point) and a point toward the objective, inside the leader's gathering ring (rotation.rs). |
| 681 | `over_water` | Commander |  | The world lately refused a drive to this tile as over the water, and no island hop is under way to cross it. |
| 688 | `home` | Commander |  | The nearest depot of my side, for resupply: of those the world has not lately refused as over the water, if any; else the nearest, and the drive's refusal sends the tank by sea. |
| 696 | `set_doing` | Commander |  | Low on what the goal needs: go home, and stay until full. |
| 703 | `why_text` | Commander | yes | A rung's reason in words, for the coordination screen: the code of `why` put into the sentence it has always been. |
| 735 | `needs_home` | Commander |  | Whether to go home to resupply: once low on shells or armour (the constants blended to the weights by `w_smooth`), until full again; held between thinks in `resupplying`. |
| 757 | `keep_inner` | Commander |  | What the inner doctrine may still do under a goal: drones always; shots at tanks unless the goal wants the gun for itself. |
| 767 | `hunt` | Commander |  | `area`: only pillboxes within this distance of this point (a wingman's, by its leader); None for the whole map. |
| 904 | `fly_wing` | Commander |  | The wingman's order of things, from Paul's notes: protect the leader; hit what is ahead of it; take the pillbox it is taking, or another near it; take a depot near it; help another ally's fight; else close up behind it. |
| 1080 | `alliances` | Commander |  | A bot seat's sides: answer what is put to it (yes to the leader and its side, yes to anyone who has not shot at it); ask to join the leader's side while not on it; and leave the side, keeping what it took, only once e... |
| 1116 | `fortify` | Commander |  | Build the castle: guard the point as `defend` does, and keep the squad on the plan whenever it is aboard and nothing threatens. |
| 1178 | `base_of` | Commander |  | The base's centre: the castle's, else the depot of my side nearest the tank when first asked, else where the tank is. |
| 1225 | `post` | Commander |  | Where the tank waits at the base: behind the gate when there is a castle, else beside the centre. |
| 1235 | `beach_worth` | Commander |  | Each beach in sight with what its island is worth as a base, in tiles of the run (`w_site_beach` times `site_score`), while afloat and the side holds no depot; none otherwise. |
| 1250 | `front_points` | Commander |  | The centres of the team board's front sectors (LR 58), with the board on; none without a board or with no hostile known. |
| 1261 | `guard_post` | Commander |  | The guard's post (backlog 51, LR 61): the post moved `w_front_post` of the way toward the nearest front sector, `FRONT_POST_REACH` at most, so an attack is met before it reaches the base and the base stays inside the... |
| 1285 | `base_threat` | Commander |  | The threat to the base, if any: the hostile tank nearest it within BASE_ALARM, and the alarm a pillbox of the base shot at raises (by a gun it may not see: indirect fire, a longshot). |
| 1311 | `neediest` | Commander |  | The base's pillbox that most wants the squad: one dead first (ours to mend, theirs to take back), then the most hurt of ours. |
| 1323 | `defend_base` | Commander |  | Defend the base: out to meet the threat as far as BASE_ALARM, else on guard at the post; the squad mends or takes back the neediest pillbox of the base whenever no hostile tank is in the leash. |
| 1362 | `raided` | Commander |  | A raid on the held ground (LR 68, `w_defend_ground`): the board's incursion, a hostile on the side's works' ground beyond the base's alarm; the side's pillbox or depot nearest it, where the guard goes, if that is an o... |
| 1378 | `outlying` | Commander |  | An outlying pillbox of mine, nearest the base: one to bring in. |
| 1390 | `turrets` | Commander |  | Turrets: fetch an outlying pillbox of mine and set it down in the next bastion. |
| 1477 | `auto` | Commander |  | The ladder: each rung's condition, top down; the first that holds (and is on) is done this tick. |
| 1599 | `slot_point` | Commander |  | My place in the leader's wedge (`WEDGE`), where it will be in a second at the leader's speed. |
| 1621 | `bridge_near` | Commander |  | A bridge under the leader, on its next three route tiles, or on the line behind it within `back` tiles (its file still on it). |
| 1638 | `helpers` | Commander |  | A fleet leader's wingmen near enough to lay its bridge. |
| 1651 | `bridge_jobs` | Commander |  | A fleet's leader, about to travel: half its wingmen within `GATHER_R`, or it holds for them (`GATHER_FOR` at most). |
| 1687 | `duck` | Commander |  | The duck (roadmap 12d, LR 19): a hostile tank with shells in direct range and line, its gun ready within `duck_ready` and before mine, and my armour share at or under `w_duck`: where to go, out of its range or line, a... |
| 1764 | `gathering` | Commander |  | The leader waits for its fleet before going out: stopped, "form up on me", for up to `GATHER_FOR`, then on with a cool-down. |
| 1799 | `expand` | Commander |  | Expand: the nearest depot not of my side to the base, its guards first; with none to be had, any pillbox on the map. |
| 1865 | `flush_say` | Commander |  | Team chat, oldest first, one line a second; a backlog of more than three is stale and dropped. |
| 1882 | `revive` | Commander |  | Revive (Paul, 16 October: "can we have an ally repair a fallen tank"): an allied tank down, not yet dead, inside `REVIVE_REACH`, with time left for the drive, the squad's walk and the work. |
| 1979 | `help_chain` | Commander |  | Chain help (Paul: "tell the bots to prioritize chain help"). |
| 2053 | `chain_target` | Commander |  | Where a chain's work is: the pillbox or depot of that id, if seen. |
| 2061 | `flank` | Commander |  | A seat's flank in a group attack: seats taken three ways, left, centre and right, each coming in sixty degrees round from the way it was coming, so three bots answering one ping close from three sides. |
| 2068 | `flank_word` | Commander |  | The seat's flank in a word, as `flank` takes it. |
| 2074 | `called` | Commander |  | What a ping marks, in a word or two: the hostile tank by it, or the place. |
| 2082 | `hear_pings` | Commander |  | A teammate's ping within `RALLY_REACH`, if this bot answers them. |
| 2132 | `call_enemy` | Commander |  | A hostile tank in sight that nobody of the side has called: ping it, and go for it too. |
| 2164 | `rallying` | Commander |  | Answering a ping: to it, and onto any hostile tank near it, with the doctrine's gun free. |
| 2268 | `fire_word` | Commander |  | The target the doctrine's gun is on and why, for `status`. |
| 2277 | `blocked` | Commander |  | No line from here: round the target to the next firing point; a whole circle and a half without one, and it is given up. |
| 2289 | `defend` | Commander |  | Hold by a point, fight what comes, mend what is ours there. |
| 2297 | `defend_within` | Commander |  | Defend `at` within `leash`: the nearest hostile tank inside it kited from the defended side, shot in direct reach or met between it and `at`; with none, to the post and on guard. |
| 2369 | `name` | Commander |  | *(the `Brain` trait's)* |
| 2373 | `settings` | Commander |  | *(the `Brain` trait's)* |
| 2377 | `set_settings` | Commander |  | *(the `Brain` trait's)* |
| 2381 | `rung` | Commander |  | *(the `Brain` trait's)* |
| 2385 | `beach` | Commander |  | The inner RuleBot's: the doctrine runs the landfall. |
| 2388 | `dead` | Commander |  | *(the `Brain` trait's)* |
| 2393 | `act` | Commander |  | *(the `Brain` trait's)* |
| 2397 | `status` | Commander |  | *(the `Brain` trait's)* |
| 2406 | `think` | Commander |  | *(the `Brain` trait's)* |
| 2410 | `think_on` | Commander |  | *(the `Brain` trait's)* |
| 2422 | `think_released` | Commander |  | The think, then `dodge_release` (lib.rs `Dodge::release`, LR 53): an errand that would drive back under a dodged shell before it lands waits, and is asked for again (`last_move` cleared); and `dodge_fire` (`Dodge::hol... |
| 2456 | `blunt` | Commander |  | Under full skill (LR 67), what the Commander says, made worse where every path to the gun and the hangar meets: a shell's aim off by up to (1 - skill) of a rookie's scatter (a direct shot a quarter of that), and no dr... |
| 2474 | `think_map` | Commander |  | The think: the inner doctrine every tick, then the goal (Auto's ladder, or the person's), and of the doctrine's orders only what the goal leaves it (`keep_inner`). |
| 3378 | `bastions` |  |  | The settings with the castle of 11b and 11h: three towers and two bastions, no cap. |
| 3386 | `one_pillbox` |  |  | Two seats, no swarm, no time limit, no pillboxes but one placed `away` tiles from seat 0 toward the middle, owned by `owner`. |
| 3404 | `run` |  |  | Run seat 0 under `goal` for `seconds`; the other seat sits still. |
| 3469 | `a_pair` |  |  | Three seats: 0 the leader, 1 its wingman on the same side, 2 the enemy; the wingman starts beside the leader, nobody else moves. |
| 3488 | `fly` |  |  | Step `w` for `ticks` with seat 1 thinking, `each` after a step. |
| 3708 | `free_near` |  |  | A free tile about `r` from `from`: open ground, off depots and pillboxes' spacing. |

Tests (24):

- `skill_spoils_the_aim_and_holds_the_drones` (line 2978): LR 67: under full skill the Commander's shells land off its aim and its drones wait; in full, nothing it says is touched; under half, no kite.
- `a_capped_castle_fills_then_lines_the_road` (line 3005): Paul's castle in play (roadmap 12h): alone, the capped castle goes up with two pillboxes inside, the first outlying pillbox is carried to the slot on its road, and the rest to posts on the road toward the side's secon...
- `a_far_fetch_gathers_the_fleet` (line 3061): The escort (roadmap 12f): with `escort` on, a leader whose fetch trip runs past `fetch_gather` waits at the base for its wingman before it goes, and the pillbox still comes home.
- `a_nan_does_not_panic_the_brain` (line 3110): Backlog 19 (LR 23): a NaN anywhere in what a tank sees used to panic the brain at the first `partial_cmp().unwrap()` that met it.
- `a_tank_ducks_a_gun_ready_before_its_own` (line 3142): The duck (roadmap 12d, LR 19): a hostile gun in range and line, ready before mine, is ducked out of range; not when my gun is ready first, not with `w_duck` at 0, not with the switch off.
- `a_teammate_finishes_a_part_laid_bridge` (line 3226): A part-laid bridge on the team board (roadmap 12f, LR 6.2): a teammate on Auto with material, nothing to do but guard, takes it and lays the rest, and it comes off the board.
- `the_posture_leans_the_ladder` (line 3284): The posture leans the ladder (LR 24): with no posture or a weight of 0 the pick is the first rung that holds, for every one of the 4096 combinations; at 0.25 a leaned rung climbs one rung, at 0.5 (and the default 0.95...
- `a_fleet_crosses_a_bridge_in_file` (line 3321): On a bridge a wingman's slot is in file straight behind the leader (roadmap 12f3); off it, the wedge; without the capability, the wedge on the bridge too.
- `a_switch_comes_back_and_a_slider_keeps_the_target` (line 3355): The sliders (backlog 15): a switch off and on again mid-match gives the role's own setting back, and a weight moved keeps the turret's target.
- `a_commander_takes_a_pillbox` (line 3438): 11a's proof: a live pillbox of the enemy's, 25 tiles off, is shot dead from outside its ring and taken by the squad inside ninety seconds, and it never lands a shell on the tank.
- `a_commander_walks_round_a_wall_to_a_line` (line 3449): A wall on the near side of the pillbox: the first firing point has no line, so the hunter walks round to one that has.
- `a_wingman_covers_its_leader` (line 3500): The wingman's first rule: a hostile tank that comes up to the leader is hit, and the wingman never leaves the leash to do it.
- `a_wingman_takes_the_pillbox_by_its_leader` (line 3549): The pillbox by the leader is the wingman's to take; one far from the leader is not.
- `a_wingman_leaves_only_at_one_hundred_percent` (line 3586): The defection gate: with one pillbox not the side's, never; with all of them and every depot, after `TOTAL_HOLD` and not before.
- `a_commander_fortifies_its_depot` (line 3622): 11b's proof: alone on the island for five minutes, the squad builds the castle round the home depot: the towers and the rear pillbox, most of the wall, the gate left open, and a route from the front into the depot com...
- `a_commander_defends_its_base` (line 3719): Defend the base: a tank parked 12 tiles out is a threat (beyond DefendDepot's leash, inside the base's alarm); it is met under the base's guns and shot, and the hurt pillbox is mended once it is gone.
- `a_commander_scoots_from_unseen_fire` (line 3764): Shelled from twenty tiles, beyond sight: the tank scoots off the spot inside a second, puts a recon up, and so sees the shooter.
- `a_bot_answers_a_chain` (line 3824): Chain help: a person who chains a pillbox alone is answered by the nearest bot of the side, which hitches on and hauls beside them, so the pillbox moves when the person drives off.
- `a_bot_revives_a_downed_ally` (line 3870): Revive: a bot whose ally goes down twelve tiles off drives over, sends its squad and has the ally back at half armour inside the twenty seconds.
- `two_crews_haul_a_pillbox` (line 3911): Haul: two crews given the same job chain an ally's pillbox and drag it to the spot, then let go.
- `a_commander_climbs_the_ladder` (line 3967): Auto, alone: fortify first; once the castle stands and the base has been quiet, the two outlying pillboxes of ours are lifted and set down in the turrets; a tank at the base sends it to defend.
- `the_ladder_in_a_match` (line 4050): The window's solo match, headless: the map in LADDER_MAP, seat 0 on Auto against three RuleBots seeded as the window seeds them.
- `a_commander_takes_a_neutral_pillbox_without_a_shot` (line 4093): A neutral pillbox is not shot at all: the squad takes it live.
- `a_commander_defends_its_depot` (line 4107): Defend depot: an intruder that drives up to the depot is engaged, and the defender never strays past the leash.

## crew.rs

The fleet commander and its crews (Paul, 16 October: "each side has a fleet commander and they request and organize fort construction").  One Commander a side commands: the leader of a bots' team, the builder of a person's. It keeps the side's castle plan (fortify.rs) as it always did, building with its own squad; what is new is that it does not build alone. It talks to the side through the world (`Command::Request`, `Event::Requested`), so it works the same over a network and every window of the side reads it on the newswire:  1. Muster, every `MUSTER_EVERY` while the plan is unfinished: "the fort is at (x, y); crews report". 2. A crew free for work within `CREW_REACH` of the fort answers Ready. 3. The commander gives each ready crew the next item of the plan nobody has (a wall, the road, a tower), or, for a crew that owns an outlying pillbox, a turret slot to bring it to (Fetch: lift it, carry it, set it down). 4. The crew does it: fells forest first when its squad is short, drives in reach, sends its squad, and when the world answers (built, refused, abandoned) reports Ready again for the next. 5. An item not done `JOB_FOR` after it was given out is given out again; one that has failed `FAILS` times the plan does without.  Who crews: rangers and a bots' team's wingmen, and a person's wingmen only while the person is at the fort, so they still go where the person goes. Guards guard.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 47 | `job_for` |  |  | How long a job has before it is late: a haul `HAUL_FOR`, the rest `JOB_FOR`. |
| 61 | `job_tile` |  |  | The tile a job works on, which no two crews are given at once; none for a job with no place (Ready, a muster). |
| 83 | `crews` | FleetCommand | yes | Crews that reported ready and are still alive. |
| 89 | `failed` | FleetCommand |  | A job not done (late, or Ready again with the item still up): one failure on its tile; `FAILS` and the plan does without it. |
| 107 | `think` | FleetCommand | yes | Every tick: the requests to send. |
| 228 | `reset` | Crew | yes | A new tank: the job it had is someone else's now (the commander gives it out again when it is late). |
| 235 | `muster_at` | Crew | yes | Where the last muster heard called the crews, if one was. |
| 240 | `hauling` | Crew | yes | What this crew is hauling, if its job is a haul. |
| 248 | `working` | Crew | yes | The crew has a job (given out, or adopted from the board). |
| 253 | `laying_road` | Crew | yes | The job is a road or a bridge tile: work wherever the fleet is. |
| 258 | `ready` | Crew |  | Report Ready to the fleet commander, if the crew has one. |
| 266 | `finish` | Crew |  | The job is over, done or not: Ready again at once if free. |
| 277 | `walk` | Crew |  | Drive to 3 tiles short of the site on the tank's side, once a second at most and not again while already going there. |
| 288 | `adopt` | Crew | yes | A job the team board kept for this crew (board.rs, roadmap 12e): taken if the crew has none and has not had this one. |
| 302 | `take_jobs` | Crew | yes | Every tick, before anything else of the bot's can return: the jobs given it. |
| 328 | `think` | Crew | yes | Every tick. |
| 523 | `ready` |  |  | Two crews answer Ready on the commander's own observation. |
| 530 | `builds` |  |  | The Build jobs among the requests: to whom, and which tile. |

Tests (1):

- `each_crew_gets_its_own_item_and_a_late_one_is_given_up` (line 544): The hand-out (backlog 26; steps 1, 3 and 5 of the module's list): a muster, then each ready crew its own item of the plan; a busy crew nothing more; a job not done in `JOB_FOR` given out again, and after `FAILS` such...

## cycle.rs

The base cycle (roadmap step 12f1, LAB-REPORT-BRAIN.md section 6.1; Paul, 18 October: "after they fill up on base resources, add to the base 1 item, 1 improvement, and attempt to build roads to other established bases or pillboxes"). A tank that has just filled up at the depot (shells, armour, and the squad's material, which a depot restocks) gives the base something before it goes back out:  1. one item of the castle plan (a wall, a tower, the road), felling first if the squad is short; 2. one improvement: the most hurt pillbox of the base mended; 3. road toward the nearest established friendly pillbox or depot not yet joined to the base, `connect_tiles` tiles a visit, so the roads grow out from the base over the match and become the way a hauled pillbox comes home (crew.rs `Haul`).  Each stage gets `STAGE_FOR` and is then passed over, so the cycle never holds a tank long; a hostile tank near ends it (the Commander checks that before asking).

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 62 | `watch` | Cycle | yes | Every tick: a fill just ended starts the cycle. |
| 70 | `begin` | Cycle | yes | Start a cycle at its first stage, the item. |
| 78 | `active` | Cycle | yes | A cycle is under way. |
| 83 | `advance` | Cycle |  | On to the next stage (item, improvement, connection, then idle). |
| 96 | `road_to` | Cycle | yes | The road tiles to lay from the base toward `to`: the straight line's grass and rubble past the yard, at most `n`. |
| 121 | `improvement` | Cycle | yes | The improvement the base most wants, by weight (`w_mend`, `w_mine`, `w_works`): the most hurt pillbox of the base mended (worth its hurt); a mine on the approach, past the wall facing the threat and off the road (wort... |
| 155 | `junction` | Cycle | yes | A road junction: a road or bridge tile with three or more road or bridge tiles beside it. |
| 167 | `road_posts` | Cycle | yes | Road posts (roadmap 12h; Paul: "they need to not stick at junctions but connect bases and castles"): on the line from the base to each of `to` (other bases and castles of the side), nearest first, a point every `spaci... |
| 200 | `think` | Cycle | yes | One think. |

Tests (4):

- `a_full_tank_gives_the_base_an_item` (line 321): One seat, its squad full: the cycle gives one plan item, then (no hurt pillbox) passes the improvement, then lays road toward the next friendly pillbox, a tile a trip.
- `the_improvement_goes_by_its_weights` (line 374): The improvement by weight: a hurt pillbox is mended before a mine at the defaults; with `w_mend` at 0 the mine; with both at 0, nothing.
- `posts_line_the_road_and_keep_off_junctions` (line 394): Posts on the line between two bases, beside the road, every spacing, none by a junction or by a pillbox already there.
- `the_road_runs_past_the_yard_and_stops_at_water` (line 426)

## facts.rs

What the map says, worked out once (roadmap steps 12b and 12f2, LAB-REPORT-BRAIN.md sections 5 and 6.2). The islands: every tile that is not water labelled with its land mass, each mass with its area and its forest; refreshed every `REFRESH`, since trees fall and bridges join masses. The crossings: `Map::crossing` is a Dijkstra over 65,536 states, and a failed one (a gap over `BRIDGE_MAX`, or no way) was asked again four times a second while the bot waited at the shore; here each answer, failures too, is kept for `CROSSING_FOR`.  And the prices that use them: a base site on a small island with few trees scores low, and a crossing is bridged or sailed by what each costs in material, a bridge past `bridge_soft` tiles costing the square of its extra length, every tree dearer where trees are scarce.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 47 | `refresh` | MapFacts | yes | Label the islands if they are older than `REFRESH`. |
| 89 | `island_at` | MapFacts | yes | The island under a point, if it is land and labelled. |
| 99 | `islands` | MapFacts | yes | Every island of the map, in id order (an id is its index and 1). |
| 105 | `crossing` | MapFacts | yes | `Map::crossing`, its answers kept for `CROSSING_FOR`, the failures with them. |
| 119 | `forget_crossings` | MapFacts | yes | Forget the crossings: the map changed under them (a bridge laid). |
| 125 | `scarcity` | MapFacts | yes | Trees are scarce here: what a base will ask for over what the island holds (LR 6.2's s). |
| 131 | `site_score` | MapFacts | yes | A base site's worth (LR 6.2): enough trees, less a small island. |
| 141 | `crossing_prices` |  | yes | What a crossing of `len` water tiles costs as a bridge and as a boat, with trees `scarcity` dear (LR 6.2). |
| 151 | `bridge_it` |  | yes | Bridge it (true) or sail it (false). |
| 162 | `island_map` |  |  | A map of water with two islands: a big forested one west, a small bare one east. |

Tests (2):

- `the_islands_are_counted` (line 186)
- `a_small_island_takes_a_boat` (line 201)

## fleet.rs

Fleets (Paul, 15 October: "can we start a game with ... 3 teams of base building, 5 on each, and 1 random aggro bot"): sixteen seats dealt into three teams of five neighbours round the map's edge and one RuleBot alone, every team's bots Commanders that share their leader's base.  The starts run north, east, south, west, the corners, then the eight between (`Map::generate`); a team is five starts side by side, so its base is in reach of all five:  ```text team 0: NW, N-west, N (seat 0, the leader), N-east, NE team 1: E-north, E (seat 1, the leader), E-south, SE, S-east team 2: S (seat 2, the leader), S-west, SW, W-south, W team 3: W-north alone, the aggro RuleBot ```  A bot team's leader is on Auto and builds the castle at its own depot; the others take its depot for their base too: a guard on defend base, and a fleet of three wingmen (two of them called rangers, crews for the castle) in a wedge on the leader. Seat 0's team assists the person in it: a builder on Auto whose castle goes round the person's depot, a guard on it, two wingmen on the person.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 56 | `word` | Role | yes | The role in a word, for the status line and the logs. |
| 69 | `teams` |  | yes | The team of every seat. |
| 75 | `role` |  | yes | A seat's role and its team's leader. |
| 83 | `role_by_rank` |  | yes | A seat's role by its place in its team, the leader first. |
| 104 | `team_role` |  | yes | Any team, as the window deals one (`--teams N`, seat i on team i % N; the default match): `team` is its seats in order, the first the leader. |
| 111 | `team_brain` |  | yes | The brain for a seat of a team dealt by the window, named for its role. |
| 137 | `set_settings` |  | yes | The process's weights for every Commander made from now on. |
| 144 | `settings` |  | yes | The weights a new Commander gets here. |
| 149 | `with_settings` |  | yes | Run `f` with this thread's Commanders made with `s`. |
| 157 | `new_match` |  | yes | A new match is being dealt: each team gets a new board. |
| 162 | `board_for` |  | yes | The board of `leader`'s team in the match being dealt. |
| 176 | `equip` |  | yes | A Commander of `leader`'s team: the weights, and the team's board if the board is on. |
| 190 | `fleet_of` |  | yes | The fleet (Paul, 17 October: "i want to see the tanks prioritise grouping and traveling in formation. |
| 201 | `name` |  | yes | The seat's name in the roster: team and role. |
| 211 | `seat_zero_team` |  |  | The seat is in seat 0's team, which the roster names as a person's to lead (a window deals it so). |
| 217 | `brain` |  | yes | The brain for a bot seat. |
| 230 | `brain_as` |  | yes | A role's brain. |
| 242 | `commander_as` |  | yes | A role's Commander, to change before it plays (a test's tactics); None for the aggro seat, a RuleBot. |
| 759 | `rung_line` |  |  | Seat-seconds a rung, in the ladder's order, for a census line. |
| 766 | `killed_by` |  |  | What took a tank, read from the events of the tick it died in (backlog 28): the weapon whose hit names it, whether it sank in a boat, and whether the shooter was the lone RuleBot. |

Tests (9):

- `three_teams_of_five_and_one_alone` (line 281)
- `a_fleet_match_plays_out` (line 300): A fleet match with no time limit is decided: one side is left inside half an hour.
- `tactics_pay` (line 330): The window's default match: the naval map, sixteen seats dealt into three teams (seat i on team i % 3), every bot in a team role and seat 0 on Auto as a person on the autopilot would be.
- `kiting_pays` (line 387): Does kiting pay (tactics.rs `kite`)? As `tactics_pay`, every team with the weights, one of the three kiting and two not, the kiting team turned round the seats over the seeds.
- `the_default_match_plays_as_teams` (line 443)
- `quad_census` (line 532): The quad match headless (scripts/quad.sh): fifteen seats, three teams, each led by a window's autopilot (a fleet commander on Auto), respawns without limit.
- `brain_census` (line 811): The brain census (roadmap 12a and 12g): sixteen seats dealt as `--fleets` deals them, one team on the trial settings and the other two on the baseline, the trial team turned round the three over the seeds so no start...
- `three_fleets_build` (line 1200): Sixteen seats as the window deals them with `--fleets`, seat 0 on Auto as a person under the coordination screen would be.
- `supply_trace` (line 1263): Trace 1 of PROPOSAL-UPGRADES.md section 6 (goal 5, 11g): supply counted and nothing bought.

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
| 393 | `turrets` | Plan | yes | The turret slots: set down, and planned (not skipped). |
| 398 | `ordered` | Plan | yes | A turret slot the squad was sent to set a pillbox down on. |
| 406 | `free_item` | Plan | yes | The first item a crew could take: pending, not a turret, not in `taken` (given out already, or the commander's own squad's). |
| 411 | `free_slots` | Plan | yes | Turret slots waiting for a pillbox, not in `taken`. |
| 416 | `own_order` | Plan | yes | The commander's own squad is on this tile: crews keep off it. |
| 421 | `give_up` | Plan | yes | A crew failed here twice: the plan does without it. |
| 428 | `gate` | Plan | yes | The gate's middle: where the road crosses the wall. |
| 434 | `castle_left` | Plan | yes | The share of the castle (gate, walls, towers) not yet built or given up: 1 just planned, 0 standing (the posture's Fortify, LR 25). |
| 444 | `slots_left` | Plan | yes | The share of pillbox slots (turrets, the road slot, road posts) still waiting for a pillbox: the posture's Connect (LR 25). |
| 453 | `done` | Plan | yes | The plan's items built. |
| 458 | `item_at` | Plan |  | The plan's item on the tile of `at`. |
| 463 | `hear` | Plan | yes | Read the world's answers to the plan's orders. |
| 537 | `next` | Plan | yes | The next order for an idle squad with `material`, or why none. |
| 588 | `progress` | Plan | yes | What the plan has, for the status line. |

Tests (2):

- `the_capped_castle_has_two_inside_one_on_the_road` (line 612): The shape, on paper: towers inside the wall and either side of the gate, every pillbox in every other's reach, the gate open, the wall on the threat's side only.
- `the_castle_is_drawn_right` (line 633)

## hop.rs

Island hopping: a bot that has taken everything it can reach goes for what it cannot, across the water.  The first naval match had a tank sit on its island for the rest of the match once the pillbox there was taken: every depot and pillbox left was over the sea, the world refused each drive to one as unreachable, and the bot waited for something to come to it. This is what it does instead, shared by the RuleBot and the Commander:  1. Idle for `HOP_IDLE` seconds (standing, the squad aboard, nothing hostile in sight) with something untaken on the map: a depot not of its side, or a pillbox or structure that is nobody's or dead. The nearest becomes the target. 2. Drive to it. The world routes by land; if it says the target is unreachable, it is over water. 3. Over water: board a moored boat within `BOAT_WALK` if there is one (`BOAT_NEAR` with a launch in hand); else drive to the bank of the launch site (the observation's water beside the bank) nearest the way to the target and launch from it (`Command::Embark`, which needs no material). If the bank is refused or not reached in `BANK_FOR`, the squad builds a boat there, for five material: short of it, restock at a depot or fell forest first. 4. Afloat, sail for the target itself: the world routes over the water and up the nearest beach to it. 5. Ashore again, the doctrine takes over within `ARRIVED` of the target; still unreachable from the new shore, back to 3. A target that took `MAX_BOATS` boats, or that even the sea does not reach, is given up.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 165 | `reset` | Hop | yes | A new tank at the start: whatever it was doing is over; what it gave up on stays given up. |
| 177 | `say` | Hop |  | Say what it is doing: the code and the words (status.rs). |
| 183 | `adopted` | Hop | yes | Laying a teammate's bridge taken from the board. |
| 188 | `active` | Hop | yes | A hop is under way: it has a target. |
| 194 | `bridge_front` | Hop | yes | The next tile of the bridge, or of the road on from it, being laid: the fleet's job (commander.rs). |
| 200 | `bridge_job` | Hop | yes | The bridge being laid, for the team board: its water tiles still to lay, in order, and what it crosses to. |
| 208 | `bridge_refused` | Hop | yes | A bridge to `target` given up here lately: not one to take. |
| 215 | `adopt_bridge` | Hop | yes | Take a teammate's part-laid bridge from the board: lay the rest of `tiles` toward `target`, and stop there. |
| 232 | `launching` | Hop | yes | Ashore, the target over the water: about to launch. |
| 241 | `send` | Hop | yes | An errand over the water (Paul's quad, 16 October: a guard whose post was across the sea drove at it for the whole match; a tank sent home to resupply sat on the wrong island at 25 armour): the world refused the drive... |
| 254 | `given_up` | Hop | yes | Given up on lately (`TRIED_FOR`). |
| 259 | `set_forget` | Hop | yes | How long what it gave up on stays given up (memory.rs). |
| 268 | `sent` | Hop | yes | The trip under way is an errand asked for (`send`), not a hop of the bot's own. |
| 274 | `give_up` | Hop |  | The target given up for `TRIED_FOR`, said why, and every part of the trip (boat, bank, bridge, road) dropped. |
| 293 | `bank` | Hop |  | The land beside a launch site, on the tank's side of it. |
| 300 | `bridging` | Hop |  | One think of the bridge: the next water tile laid from the end so far, the tank at that end, the material felled or fetched. |
| 404 | `road_line` | Hop |  | The road on from a bridge to the target: the grass and rubble tiles on the straight line between, `ROAD_ON` at most, the last few short of the target left for it. |
| 431 | `lay_road` | Hop |  | Lay the road on, a tile ahead of the tank each time; true while it does. |
| 482 | `landing` | Hop |  | The beach to land on for `target`, if one in sight is within `LANDING_NEAR` of it: distance to the target plus its danger, the choice kept unless another is clearly better (`landing_at`). |
| 517 | `covered` | Hop |  | What covers the water here, if anything (`COVER_R`). |
| 532 | `pick` | Hop |  | Something untaken that is not where the tank is: the nearest. |
| 542 | `taken` | Hop |  | A depot or live pillbox of the side stands at `at`: the hop's target is taken. |
| 549 | `hear` | Hop | yes | Every tick, think or not: the world's answer to an order comes the tick after it, which is rarely a think tick. |
| 602 | `think` | Hop | yes | Every think. |
| 889 | `across` |  |  | A hop sent across a channel `wide` tiles of water, the map grass round it and 200 tiles of forest behind: its first think, and the Hop after. |

Tests (1):

- `a_hop_bridges_a_narrow_gap_and_sails_a_wide_one` (line 925): The bridge choice (backlog 26; Paul, 17 October: "build roads where it would drive"): a gap of `BRIDGE_MAX` tiles or fewer is bridged from the near bank, a wider one sailed; priced (LR 6.2), a bridge that costs more t...

## intent.rs

Holding a decision (roadmap step 12d, LAB-REPORT-BRAIN.md section 7). A tank keeps doing its last command until a new one has been confirmed over several thinks; only the survival reflexes, dodging a shell and going home to resupply, break in at once and at full throttle. Two pieces:  - `Desire`, a smoothed goal: D = (1 - alpha) D + alpha d. A goal that moves a little each think (a wedge slot ahead of a moving leader, a rally on a moving enemy) is followed smoothly instead of re-routed every tick; a jump past `jitter` tiles is taken at once. - `Confirm`, a Schmitt trigger on a choice: a challenger replaces the held choice only when it beats it by `margin` for `need` thinks in a row, and the held choice's score carries a commitment bonus.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 26 | `toward` | Desire | yes | Where to go this think, given where the raw pull says; `alpha` the share of the new pull, `jitter` the jump taken at once. |
| 36 | `reset` | Desire | yes | Forget: the next pull is taken as it is (a dodge, a new goal). |
| 41 | `at` | Desire | yes | Where it went last, if it has a pull. |
| 54 | `default` | Confirm |  | *(the `Default` trait's)* |
| 63 | `choose` | Confirm | yes | This think's best `best` (score `best_s`) against the held choice (score `held_s`, None if it is no longer possible): what to do. |
| 96 | `force` | Confirm | yes | Take `x` now, whatever the score: a survival reflex, or an order. |
| 102 | `held` | Confirm | yes | The choice held, if any. |
| 107 | `clear` | Confirm | yes | Hold nothing: the next choice is taken at once. |
| 130 | `update` | Advance | yes | Smooth in `raw` (clamped to -1..1) at share `beta` and read the step: push above `band`, back below `-band`, else hold. |

Tests (3):

- `a_challenger_needs_two_thinks` (line 147)
- `a_dodge_skips_confirmation` (line 166)
- `the_advance_holds_between_its_bands` (line 178)

## learn.rs

Training the learned tank (LR 95, 97): the phases' training grounds and evolution strategies over a `Net`'s weights, in plain Rust on threads.  A ground is one tank on an open square of grass with a target: an enemy tank standing still, a neutral depot, or a neutral pillbox (phase 1, go to), or an enemy tank to hit (phase 3, aim). An episode is 60 s at most; its score is what the tank earned (`Task::score`).  Evolution strategies (Salimans and others 2017): each generation draws pairs of random directions in weight space, plays the weights a little way along each and a little way back on the same seeds, ranks every score, and moves the weights toward the directions that scored better (Adam). No gradients and no GPU: every episode is its own, so every core is filled.  `make train PHASE=1` runs `train` (an ignored test, as the census is); `make train PHASE=1 CHECK=nets/phase1.bin` reads a net on held- out seeds against its phase's pass (`check`).

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 56 | `phase` | Task | yes | The task a phase trains (`make train PHASE=`), if it has one yet. |
| 65 | `target` | Task | yes | The target a seed's ground has: the three in turn. |
| 70 | `shoots` | Task | yes | Whether this target is shot (else gone to). |
| 75 | `distance` | Task |  | The target's distance at the start, tiles. |
| 95 | `ground` |  | yes | The ground for a task and seed: the seed's generated map with a square of grass `GROUND` tiles each way round its centre, the tank at the centre facing a way the seed draws, the target 10 to 20 tiles off in a directio... |
| 171 | `score` | Task | yes | An episode's score, the reward summed. |
| 191 | `passed` | Task | yes | Whether an episode passed its phase's test: within `REACH` inside the minute; a direct hit on the enemy inside the minute. |
| 198 | `play` |  | yes | One episode of a brain on a ground. |
| 203 | `play_on` |  | yes | As `play`, recording frames into `rec` (the clips, LR 102). |
| 250 | `normal` |  |  | A normal draw (Box and Muller). |
| 257 | `direction` |  |  | The noise of one pair of a generation: a direction, from its seed. |
| 263 | `spread` |  | yes | Run `jobs` jobs on `threads` threads, results in order. |
| 295 | `parse` | Course | yes | `phase1`, `phase3`, or a scenario file's path. |
| 313 | `name` | Course | yes | The course's name in the trainer's lines: the task, or the file's stem. |
| 322 | `floor` | Course | yes | The share of held-out episodes it must pass: a file's `floor`, else 95%. |
| 332 | `scripted` | Course | yes | The same file with its net seats given to the scripted brain (`kind = "commander"`, on Auto): what the scripted brain does on the same situations. |
| 353 | `prepare` |  | yes | Episode `k` of a generation: the courses in turn. |
| 361 | `episode` |  | yes | A net on one episode: its score, and whether it passed. |
| 376 | `judge` |  | yes | A net on episodes: the mean score, and the share passed. |
| 412 | `new` | Adam | yes | Moments at zero for `n` weights. |
| 417 | `step` | Adam | yes | Move `theta` up the gradient `g`. |
| 445 | `generation` |  | yes | One generation: pairs of directions played forward and back on `seeds` episodes drawn from `gen_seed` (the courses in turn, so a mix rehearses what was learned before), ranked, and the weights moved. |
| 490 | `check` |  | yes | A brain's reading on held-out grounds: for each target, the share passed and its standard error, and the mean score. |
| 542 | `name` |  |  | *(the `Brain` trait's)* |
| 545 | `think` |  |  | *(the `Brain` trait's)* |
| 556 | `name` |  |  | *(the `Brain` trait's)* |
| 559 | `think` |  |  | *(the `Brain` trait's)* |
| 631 | `env` |  |  | A setting from the environment, or `d` when unset, empty or unread. |
| 637 | `report` |  |  | Prints a net's reading on held-out grounds, each target's share passed with its standard error. |
| 666 | `baseline` |  |  | The scripted bot on the same held-out grounds: its share passed and, shooting, its hits a shot (a reference, LR 97). |
| 681 | `report_file` |  |  | A training map read on held-out seeds, by a net or (with none) by the scripted brain in its seats: the share passed with its error, the mean reward, and whether it reaches the map's floor. |
| 707 | `report_all` |  |  | A net (or with none, the scripted brain) on every course's held-out seeds. |
| 734 | `match_minutes` |  |  | The match reading (LR 100): seat 9 of a dealt match (a ranger of team 0, whom nobody follows) given to the net, or kept by its scripted brain, on a map from the seed, three respawns, for `secs`: when the seat first we... |
| 773 | `match_reading` |  |  | `MATCH=n`: the match reading over n seeds from 8,000,000, the net (`CHECK`) and the scripted ranger on the same seeds. |
| 797 | `name` |  |  | *(the `Brain` trait's)* |
| 800 | `think` |  |  | *(the `Brain` trait's)* |
| 834 | `sim_dodge` |  |  | The bound by simulation (LR 100): map 5's situation built here (the tank rolling at full speed on a heading the seed draws, a shell landing within 2.5 tiles of it in 1.5 to 3.5 s), and every 0.2 s each of the nine dri... |
| 918 | `probe` |  |  | `PROBE=oracle GROUND=map`: the oracle in the net's seat on the held-out seeds. |
| 935 | `gauntlet` |  |  | `make gauntlet` (LR 102): every file in `scenarios/gauntlet/`, for the scripted brain, the untrained net and each milestone net, on the same `GAUNTLET_RUNS` seeds (200) from 9,500,000: the share passed with its error,... |
| 948 | `gauntlet_of` |  |  | The gauntlet for these brains (None the scripted brain), to `out/file`. |
| 983 | `gif` |  |  | A recording drawn and made a GIF (LR 102): each frame to a PPM, then `ffmpeg` with a palette of its own. |
| 1008 | `match_clip` |  |  | The match clip (LR 102): seat 9 of a dealt match given to `net`, recorded round its start for `secs`. |
| 1052 | `clips` |  |  | `make clips` (LR 102): a short GIF of each milestone at work, on its defining test, the first held-out seed it passes (the untrained net on the first seed), and the newest net in a real match. |
| 1120 | `timing` |  |  | `TIMING=1` (LR 102): what the learned tank costs alone, on one thread: a decision (the senses and the forward pass); a one-tank episode of phase 3's ground; and 120 s of a 16-seat match with every seat a net against e... |
| 1172 | `read_course` |  |  | A course read on its held-out seeds by a net: the share passed (for a built ground the lowest of its targets'), its error, and the course's floor (LR 103). |
| 1202 | `school` |  |  | `make school` (LR 103): the learned tank's curriculum, run until stopped, as `make converge` runs the scripted brain's census. |

Tests (5):

- `a_ground_is_open_and_its_target_is_there` (line 517): The grounds: the tank at the centre on grass, the target where the ground says and of its kind, open ground between, and a seed the same ground twice.
- `a_straight_drive_reaches_and_a_still_tank_does_not` (line 539): The ground is fair: a brain that drives straight at the target reaches each kind well inside the minute; one that holds never does; and an enemy shot at from inside the gun's reach is hit.
- `a_training_map_is_a_course` (line 582): The training maps (LR 99): every file under `scenarios/training/` reads as a course (a net seat, known reward terms); a net plays one the same way twice; its scripted reading puts a Commander in the net's seat; a rewa...
- `evolution_moves_the_weights_the_same_way_twice` (line 610): Evolution moves the weights: on a few grounds, a handful of generations raise the score, and a generation's seed gives the same move twice.
- `train` (line 1316): The trainer (LR 97, 99): `make train PHASE=1`, or `GROUND=` a comma list of courses (`phase1`, `phase3`, training maps' paths) trained together, each episode the next course in turn.

## ledger.rs

The ledger (roadmap step 12g, LAB-REPORT-BRAIN.md section 8). Every census match writes one JSON line per seat: the genome (the weights, the capabilities, the seed, the role) beside what the seat did, so a high score can be played again (the sim is deterministic: the line and the map are the match) and its weights bred from (`scripts/tune.sh`).  `Tally` counts as the match runs, from the world's events and, once a second, from what each team holds; `records` writes the lines. Ratings are Elo over the ledger's matches: a genome's team placed above another's in a match counts as a win against it, so a lucky score against weak opponents counts for less than a steady one against strong ones.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 33 | `of` | Genome | yes | The genome of these Settings in this role, its id their hash. |
| 125 | `default` | ScoreWeights |  | *(the `Default` trait's)* |
| 148 | `from_long_game` | ScoreWeights | yes | The score between elimination and holding ground (`long_game`, roadmap 12h): 0 weighs the win and kills half as much again and the holdings not at all; 1 the win and kills half as much and the holdings twice; 0.5 is t... |
| 155 | `without_rewards` | ScoreWeights | yes | The score before the rewards (LR 8 to 83): `CENSUS_REWARDS=0`. |
| 161 | `rewards` |  | yes | The rewards' part of a seat's score (LR 84). |
| 168 | `score` |  | yes | A seat's match score: the win, kills less deaths, ground held over the match's length, works built and road laid, and the rewards, each at its weight. |
| 188 | `new` | Tally | yes | An empty count for each seat of the world, with its team. |
| 198 | `hear` | Tally | yes | After each `step`: the tick's events; once a second, the holdings. |
| 293 | `records` | Tally | yes | The match is over (or out of time): a line per seat with a genome. |
| 310 | `write` |  | yes | Append records to `dir/ledger.jsonl`. |
| 321 | `read` |  | yes | Read a ledger; bad lines are skipped. |
| 328 | `mean_se` |  | yes | The mean of a sample and its standard error (the sample's standard deviation over the root of its size); the error is 0 for fewer than two. |
| 344 | `ratings` |  | yes | Elo ratings by genome over the ledger's matches: within a match, each genome's best seat score against every other genome's, a win for the higher. |

Tests (4):

- `ledger_ratings` (line 396): The hall of fame: the ledger's genomes by Elo, best first, with the weights each changed (`LEDGER`, default reports/ledger/ledger.jsonl).
- `the_standard_error_of_a_sample` (line 408)
- `a_ledger_line_round_trips_and_rates` (line 418)
- `the_rewards_count_their_events` (line 448): Each reward counts what it says, for the seat that did it, and only against a hostile (LR 84); `without_rewards` is the score before them.

## lib.rs

treadcore-brain -- the controller interface, and one controller.  The prior art called them Brains: a program that takes the tank's place at the controls, given only what the tank can see. Same idea here. A Brain gets an `Observation` -- the player's view of the world, filtered by the host, nothing a human client does not also get -- and returns `Command`s, which the world validates like anyone else's. A Brain cannot cheat because it has no more access than a keyboard.  Three implementations: `RuleBot`, which fills empty seats and is good enough to kill a careless player; `Commander` (`commander.rs`), which drives the player's own seat toward a goal the player chose; and `JsonBrain`, which speaks the observation and commands as JSON over stdin/stdout so an external program -- a Python script, an LLM harness -- can play without linking this crate.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 49 | `name` |  |  | The seat's name, as the scoreboard and the logs show it. |
| 53 | `think` |  |  | One tick's orders for the seat from what it can see: called every tick the tank is alive (`run_bots`); a brain that thinks less often answers with nothing in between. |
| 57 | `think_on` |  |  | `think` with the map (the host's world map, or a window's copy): public, as every window draws it, and what plans a bridge or a road needs (hop.rs). |
| 62 | `status` |  |  | What it is doing, in a line, for a window that shows its fleet. |
| 67 | `settings` |  |  | Its weights and capabilities, for the ledger (weights.rs); None for a brain without them. |
| 72 | `act` |  |  | What it is doing, as a code (status.rs): for logic and the census, where `status` is for people. |
| 77 | `rung` |  |  | The Auto rung this think took, if it reached the ladder (the census, LR 26); None for a brain without one. |
| 82 | `beach` |  |  | Afloat: the beach it is running for, if any (the census's read of where the gun stands, backlog 34). |
| 87 | `dead` |  |  | Each tick the seat is dead, in place of a think: what a brain counts of its fall (the dodge's meter, LR 27). |
| 90 | `set_settings` |  |  | New weights mid-match (the coordination screen's sliders, backlog 15); a brain without them ignores it. |
| 97 | `retire_taken` |  | yes | A person took a bot's seat (roadmap step 8): that bot stops, from the tick the world said so. |
| 108 | `run_bots` |  | yes | Run every bot seat for one tick: each seat's own Observation, a `think_on` with the world's map if alive, `dead` and a respawn when due if not. |
| 161 | `update` | Grace |  | Note every hostile tank seen below full fuel or reloading: it has moved or fired. |
| 174 | `fair` | Grace |  | A fair target: the first minute is over, or the tank has been seen to move or fire. |
| 363 | `watch` | Dodge | yes | The meter's reading of the dodge under way: hit (or killed) between its start and its shell's landing, or clear of it. |
| 375 | `order` | Dodge | yes | The dodge's order this tick, if any: out of a seen shell's way, or a jink after a hit. |
| 382 | `order_with` | Dodge | yes | `jink`: answer a hit with a jink too. |
| 392 | `order_on` | Dodge | yes | As `order_with`, with the map: the dodge point is one the tank can reach (passable ground; afloat, water or a beach). |
| 453 | `threats` | Dodge |  | The landings a dodge plans against, each where and in how many ticks: with `dodge_every` (LR 56) every hostile indirect shell in flight landing within the horizon, else `dodged` alone (the one it dodges; for the relea... |
| 471 | `clearance` | Dodge |  | The least clearance of a drive over the landings: at each, its distance from that impact less the splash and the margin; below 0, inside one. |
| 486 | `planned` | Dodge |  | `dodge_plan` (LR 53): the dodge point by where the tank will be when the shell lands. |
| 519 | `release` | Dodge | yes | `dodge_release` (LR 53): a tank at its dodge point before the shell lands is let go to its errand at once (`dodge_hold` 0), and errands often drove it straight back under the shell. |
| 539 | `hold_fire` | Dodge | yes | `dodge_fire` (LR 57): a tank has no turret; a shot turns the hull to face it (treadcore-sim, `FireDirect` and `FireIndirect`), and at speed the tank runs on that way. |
| 564 | `holding` | Dodge | yes | A dodge under way: hold it while the tank drives to its point, and, by `dodge_hold`, that share of the shell's flight once there (standing out of the splash rather than back to its errand under the shell). |
| 575 | `lead` |  | yes | Where to aim a direct shot at a moving tank: where it will be when the shell lands (40 tiles a second, 0.8 tiles of hit radius; a tank at full speed moves a tile in the flight from 10 tiles off, so a shot at where it... |
| 595 | `drive_to` |  | yes | Where a tank will be after `ticks` sent to `to`, by the sim's own rules of motion (treadcore-sim `drive_tank`): the route the world will plan (`Map::route_by`: eight ways from tile to tile, a waypoint at each turn, so... |
| 602 | `drive_path` |  | yes | As `drive_to`, where the tank is after each of the `ticks`: element k after k + 1 ticks. |
| 614 | `drive_path_by` |  | yes | `drive_path`; with `own` (`dodge_own_route`, LR 93), a drive ashore to where the tank's route already ends follows that route, its waypoint and the rest: the way the world will drive it (pillboxes' reach and mines pri... |
| 709 | `update` | Form |  | Look again if it is time. |
| 738 | `patience` | Form | yes | The patience, scaled: bold waits less. |
| 742 | `standoff` | Form | yes | The firing position's distance, scaled: careful stands further off. |
| 746 | `home_armor` | Form | yes | The armour below which the bot goes home: careful goes sooner. |
| 780 | `hurt_by_player` | RuleBot | yes | Has this player's fire hit the bot's tank this match? |
| 787 | `new` | RuleBot | yes | A RuleBot at full skill, every part on: `seed` deals its own randomness (its patience among it), so two bots of one seed play alike. |
| 797 | `remember_guns` | RuleBot |  | Hostile tanks kept where last seen, for `landfall_memory` seconds (backlog 36): one seen now is kept where it is; a memory goes when its time is up, or when its spot is in the team's sight and the tank is not there. |
| 813 | `known_guns` | RuleBot |  | The hostile tanks the landfall prices: those seen now and those remembered (`landfall_memory`). |
| 818 | `standing_off` | RuleBot | yes | Afloat and standing off a covered beach (`landfall_wait`). |
| 828 | `landfall_order` | RuleBot |  | Afloat (SPEC TNK-08): run for land. |
| 918 | `is_rookie` | RuleBot | yes | The same doctrine, badly (PLAYABILITY item 9): thinks half as often, launches nothing for `ROOKIE_LATE` seconds and then keeps one interceptor up, aims where the tank is with `ROOKIE_SCATTER` tiles of error and no lea... |
| 925 | `aim_skill` | RuleBot |  | The skill its aim is spoiled by: a Commander's doctrine (with `tactics`) aims in full, its Commander spoiling every shot once (`Commander::blunt`). |
| 935 | `rookie` | RuleBot | yes | A RuleBot at skill 0 (`is_rookie`): no dodge, kite or storm, one interceptor up and late, a slower think, its shells scattered. |
| 940 | `without_pillboys` | RuleBot | yes | The same doctrine without the squad: the control group. |
| 946 | `nearest_enemy` | RuleBot |  | The nearest hostile tank in sight that the first minute lets it shoot (`Grace::fair`). |
| 959 | `works_order` | RuleBot |  | The squad's works (steps 10i and 10j), in this order: a power plant when a structure of this bot's stands dark with no plant of the side in reach (everything else of the economy is dead without one); a jammer once a s... |
| 985 | `works_need` | RuleBot |  | What this bot's structures within `reach` want built beside them, first want first: a power plant for a dark one with no plant of the side in reach; a jammer, once the side has been hit, for one no jammer covers; a wo... |
| 1004 | `fell` | RuleBot |  | Scrap: the nearest forest in reach, if the squad has room for it. |
| 1013 | `nearest_depot` | RuleBot |  | The nearest depot of the side (or not), of those the world has not refused a drive to as over the water. |
| 1022 | `name` | RuleBot |  | *(the `Brain` trait's)* |
| 1027 | `beach` | RuleBot |  | The beach `landfall_order` chose, while afloat. |
| 1031 | `status` | RuleBot |  | *(the `Brain` trait's)* |
| 1039 | `think` | RuleBot |  | *(the `Brain` trait's)* |
| 1043 | `think_on` | RuleBot |  | *(the `Brain` trait's)* |
| 1053 | `think_map` | RuleBot | yes | The think, with the map when there is one: every tick, what it hears (hurt, refused, guns seen); then the dodge, and on its think tick the doctrine's ladder from fighting to resupply. |
| 1678 | `new` | Defender | yes | A Defender of the depot it starts on; `fortify` false is the control (holds and fights, builds nothing). |
| 1706 | `ring` | Defender |  | The ring of tiles at `r` round `c`, in the order to lay them: the far side first. |
| 1729 | `mouth_sites` | Defender |  | Candidate sites for the mouth pillbox: along the gate direction at `MOUTH`, then a little to either side, then further out. |
| 1742 | `approach_sites` | Defender |  | The approach mines: three across the gate direction at `APPROACH`, the road's tile in the middle. |
| 1749 | `name` | Defender |  | *(the `Brain` trait's)* |
| 1752 | `think` | Defender |  | *(the `Brain` trait's)* |
| 2034 | `spawn` | JsonBrain | yes | Start `program` with `args` and read its answers on a thread of their own, so a slow program never holds up a tick. |
| 2058 | `name` | JsonBrain |  | *(the `Brain` trait's)* |
| 2061 | `think` | JsonBrain |  | *(the `Brain` trait's)* |
| 2078 | `drop` | JsonBrain |  | *(the `Drop` trait's)* |
| 2084 | `schema_examples` |  | yes | Example documents for the JSON brain, printed by the server. |
| 2649 | `name` |  |  | *(the `Brain` trait's)* |
| 2652 | `think` |  |  | *(the `Brain` trait's)* |

Tests (20):

- `a_bot_drives_out_from_under_a_shell` (line 2116): PLAYABILITY item 9: a rookie is beatable.
- `a_dodge_holds_out_of_the_splash_by_its_weight` (line 2143): `dodge_hold` (LR 27): a tank at its dodge point before the shell lands stays there at 1, and is free to its errand at 0 (the dodge before LR 27).
- `drive_to_foresees_the_world` (line 2171): `drive_to` (LR 53) foresees the world: a tank at rest or rolling, on four headings, sent to twelve points round it 5 tiles off, is where `drive_to` said after 2 s, within a tenth of a tile.
- `the_own_route_foresees_a_drive_round_a_gun` (line 2208): `dodge_own_route` (LR 93): with a hostile pillbox beside the way, the world's route goes round its reach, and the forecast along the tank's own route says where the tank is after 3 s; a searched route, which leaves th...
- `a_shy_boat_runs_for_the_beach_clear_of_the_guns` (line 2242): `landfall_shy` (backlog 30, LR 31): of two beaches, the nearer one's run passing a hostile gun in reach, at 0 the boat takes it (its price is the beach and the heading only), at 1 the farther one clear of the guns.
- `a_boat_lands_where_the_island_is_worth_the_run` (line 2265): `beach_worth` (backlog 20, LR 62): of two clear beaches, with no worth the boat takes the nearer; the farther worth more than the difference in the run, the farther.
- `a_boat_stands_off_a_covered_beach_for_its_wait` (line 2287): `landfall_wait` (backlog 33, LR 33): the only beach covered by a seen gun 8 tiles off, at 0 the boat lands; at 10 it backs off out of the gun's reach, and lands once the 10 s are spent.
- `a_gun_seen_and_lost_still_prices_the_run` (line 2314): `landfall_memory` (backlog 36, LR 36): a gun seen beside the near beach's run and then lost from sight still prices that run for its memory's seconds (here with `landfall_shy` 1), and not after.
- `rulebot_kiting_pays` (line 2356): RuleBots win more than twice as often, kill more and outlive them, and the rookies still kill something.
- `rookies_are_beatable` (line 2401): *(the `Drop` trait's)*
- `a_rulebot_drives_to_its_zone_to_build` (line 2441): Step 10k: a RuleBot with the scrap drives to a dark refinery of its own thirty tiles off, out of the squad's reach, and powers it.
- `a_rulebot_powers_its_zone_then_builds_a_workshop` (line 2477): Step 10j: a RuleBot holding a dark refinery builds a power plant beside it first, then a workshop; no jammer, because nothing of its side has been hit.
- `a_rulebot_jams_after_a_hit` (line 2512): Step 10i: a RuleBot holding a powered zone builds no jammer until a structure of its side is hit; then it builds one on a free pad, launches nothing inside it, and drives out of it.
- `a_rulebot_answers_a_person` (line 2568): A RuleBot answers a person's proposal: yes, unless that person has hurt it.
- `pillboys_pay` (line 2602): Sixteen bots, six minutes: the rules get exercised.
- `fortification_pays` (line 2642): Roadmap step 6c's proof: a fortified depot survives a scripted attack longer than a bare one, over twenty seeds.
- `a_newcomer_gets_a_first_minute` (line 2725): PLAYABILITY.md item 1, the first minute: a newcomer who sits at the start reading the help is not shelled by the bots for the first minute.
- `defenders_hold` (line 2767): Roadmap step 6d's proof, as it came out: sixteen bots, four attacking, over ten seeds of five minutes.
- `bots_fight_to_a_result` (line 2848)
- `a_led_direct_shot_cannot_be_dodged` (line 2879): Roadmap 12d, backlog 8 (LR 19): a direct shell flies five ticks at most (10 tiles at 40 a second), and a tank at full speed turning as hard as it can moves about 0.4 tiles off the gunner's lead in that time, inside th...

## memory.rs

Memories that run out (roadmap step 12b, finding L6 of LAB-REPORT-BRAIN.md). The brain kept its given-up targets in lists that grew all match and were never looked at again: a pillbox walked round twelve times stayed given up after its guards were dead, a depot refused once over the water stayed refused after the bridge was laid. `Forget` keeps each with the tick it was put there, and a memory older than its `keep` is gone. A `keep` of `NEVER` remembers for the match, as the lists did; that is the default until the census promotes a time (the `FORGET` capability, weights.rs).

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 24 | `default` | Forget |  | *(the `Default` trait's)* |
| 31 | `new` | Forget | yes | Nothing remembered; each memory kept `keep` ticks (`NEVER`: for good). |
| 36 | `live` | Forget |  | A memory put at `at` is still kept at `now`. |
| 41 | `put` | Forget | yes | Remember `x` from `now`; remembered already, the clock restarts. |
| 50 | `has` | Forget | yes | Still remembered at `now`. |
| 55 | `forget_old` | Forget | yes | Drop what has run out. |
| 63 | `clear` | Forget | yes | Forget everything. |
| 68 | `remove` | Forget | yes | Forget `x` now, its time up or not. |
| 73 | `len` | Forget | yes | How many are held (old ones included until the next `put`). |
| 78 | `is_empty` | Forget | yes | Nothing held (old ones included, as `len`). |

Tests (1):

- `a_memory_runs_out_unless_kept_for_ever` (line 88)

## meter.rs

What the brain spends (roadmap step 12a, LAB-REPORT-BRAIN.md section 5, "measure first"). Counters, one set per thread, so the census tests that run side by side each count their own match: thinks, the Moves the bots asked for, the crossing searches, the plan copies, the crossing answers the cache gave instead, and the time `run_bots` took. Nothing reads them but a census and `Brain::status`-style reports; no decision depends on them, so a replay is the same with them or without.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 117 | `of` | SquadTask | yes | The squad's task read: `water` whether the site is water or a bridge (a road there is a bridge). |
| 151 | `add` | Dodges |  | Another count added to this one. |
| 168 | `trial` |  | yes | The trial seats, a bit a player; the rest are group 0. |
| 172 | `group` |  |  | The group of the seat being run: 1 if `trial` marks it, else 0. |
| 180 | `bump` |  |  | Change this thread's counts. |
| 189 | `seat` |  | yes | A seat run, the player's: the counts after it are its group's. |
| 194 | `think` |  | yes | A RuleBot think (four a second at full skill). |
| 198 | `moves` |  | yes | Move orders given. |
| 202 | `crossing` |  | yes | A water crossing worked out. |
| 206 | `crossing_cached` |  | yes | A water crossing answered from the facts' cache. |
| 210 | `plan_clone` |  | yes | A fortification plan copied. |
| 215 | `posture` |  | yes | A ladder pick under a posture: the rung the ladder alone takes (`first`) and the one taken with the posture's lean. |
| 225 | `dodge` |  | yes | A shell dodge begun, ashore or afloat, taken or dropped (LR 27). |
| 231 | `dodge_hit` |  | yes | A dodge hit before its shell landed: killed by it, and by the dodged shell itself (the hit on its landing tick). |
| 242 | `front` |  | yes | An Expand pick with `w_front` on: the front known or not, and moved from the nearest depot or not. |
| 255 | `lean` |  | yes | A second of the rotation's push point (`post` false) or the guard's post (`post` true) worked out: moved toward the front or not. |
| 260 | `beach_site` |  | yes | A second afloat with the beaches priced by their islands. |
| 265 | `base_site` |  | yes | A base chosen by its site, `off` tiles from the nearest depot. |
| 276 | `kite` |  | yes | A second alive: kiting at `step`, or not. |
| 296 | `squad` |  | yes | A second alive: the squad out at `task`, or aboard (None). |
| 306 | `spend` |  | yes | The commands of a think: its shots and launches counted. |
| 326 | `ground` |  | yes | A board's second: an incursion on the held ground, and Defend held only for it. |
| 334 | `spent` |  | yes | Time spent running the bots. |
| 339 | `read` |  | yes | This thread's counts. |
| 344 | `reset` |  | yes | Start counting again from zero. |
| 350 | `add_dodges` | Meter | yes | Adds another count's dodges to this one's (a census's total). |
| 361 | `add_fronts` | Meter | yes | Adds another count's front picks to this one's. |
| 408 | `dodge_line` | Meter |  | The dodges for the meter's line, each kind that happened. |
| 427 | `micros_per_seat` | Meter | yes | Microseconds per seat-tick: what one bot costs a tick, on average. |
| 436 | `line` | Meter | yes | One line for a census: the counts and the cost. |

Tests (4):

- `the_meter_counts_and_resets` (line 477)
- `a_squad_second_counts_by_its_task` (line 494): A squad's second counts under its seat's group, by its task, a road over water as a bridge (LR 83).
- `a_dodge_counts_by_its_seat_and_whether_taken` (line 517): A dodge counts under the seat that made it (the trial's or the rest), taken or dropped, ashore or afloat (backlog 31).
- `a_second_alive_counts_its_kite_step` (line 537)

## net.rs

A learned tank (LR 95, 96): a small net from the senses (senses.rs) to an action, and the Brain that drives by it.  The net is a multilayer perceptron: each layer multiplies by a table of weights, adds a bias, and bends the result (`max(0, x)`) in every layer but the last. Its outputs are scores for the action heads (`Head`); in each head the highest score among the choices allowed now (the mask) is taken. Plain Rust, no framework: the census and the trainer run it on any machine.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 103 | `random` | Net | yes | A net of the given sizes (inputs, hidden..., outputs), its weights drawn from a seed: uniform within +/- sqrt(6 / (in + out)), so a layer's outputs start the size of its inputs (Glorot and Bengio 2010); biases 0. |
| 123 | `tank` | Net | yes | The first nets' shape: the senses, `HIDDEN`, the heads. |
| 133 | `widen` | Net | yes | The same net taking `inputs` inputs, the new ones last with zero weights: it computes exactly what it did on the inputs it had (LR 100: a net of the 133 senses before the shell contact). |
| 149 | `parameters` | Net | yes | Its weights and biases in one count. |
| 154 | `forward` | Net | yes | The outputs for these inputs. |
| 172 | `flat` | Net | yes | Every weight and bias, layer by layer, as one vector (for a trainer that moves them all at once). |
| 177 | `with_flat` | Net | yes | The same shape with these weights (`flat`'s order). |
| 196 | `save` | Net | yes | Written as: the magic, the count of sizes, the sizes (u32), then each layer's weights and biases (f32), all little-endian. |
| 213 | `load` | Net | yes | A net read from `save`'s form; an error for a file that is not one, or whose weights do not fit its sizes. |
| 237 | `choose` |  | yes | Each head's choice: the highest score among those `allowed`. |
| 269 | `new` | NetBrain | yes | A brain driven by `net`, which must take the senses and score the heads; its own queue of calls until `with_calls`. |
| 279 | `with_calls` | NetBrain | yes | The same, sharing a team's queue of calls (one a team a match). |
| 287 | `call_at` | NetBrain |  | Where a call of this kind would point: me for help, the nearest enemy seen (tank, else live hostile pillbox) for attack, the nearest thing to take for take. |
| 297 | `tank_in` | NetBrain |  | The strongest hostile tank in sector k, if any. |
| 304 | `gun_target` | NetBrain |  | What the gun fires at in sector k: its nearest hostile tank in the direct gun's reach (with the tank, for the lead), else its nearest live hostile pillbox in reach (a pillbox sits still). |
| 322 | `launch` | NetBrain |  | A drone launch for the drone head's choice c (1 interceptor, 2 recon, 3 strike, 4 longshot), if the bay has one and it has something to do: an interceptor while fewer than `BUBBLE_FULL` are up; a recon while none of m... |
| 347 | `mortar_target` | NetBrain |  | The strongest hostile in sector k the mortar can reach: a tank, else a live hostile pillbox. |
| 356 | `name` | NetBrain |  | *(the `Brain` trait's)* |
| 360 | `think` | NetBrain |  | *(the `Brain` trait's)* |
| 364 | `think_on` | NetBrain |  | *(the `Brain` trait's)* |
| 436 | `status` | NetBrain |  | *(the `Brain` trait's)* |

Tests (7):

- `a_net_is_its_seed_and_its_file` (line 449): The net: its shape (about 13,000 weights), the same outputs for the same seed, other outputs for another, and a file that reads back as the same net.
- `the_gun_takes_a_hostile_pillbox_in_reach` (line 469): The gun's target (LR 99): a live hostile pillbox in reach and in the sector when no tank is; never a neutral one, a dead one, or one out of reach.
- `a_widened_net_is_the_same_net` (line 492): A widened net (LR 100) computes what it did: the old inputs with zeros after give the old outputs.
- `the_drone_head_launches_what_it_can` (line 506): The drone head (LR 100): an interceptor while the bubble is short and the bay has one; none with the bay empty; a strike only at a hostile tank in reach; a recon only while none is up.
- `a_masked_choice_is_never_taken` (line 527): The mask: a head's choice is never one not allowed; hold always is.
- `a_random_net_drives_a_tank_the_same_way_twice` (line 543): A net drives a tank: random nets seated in a world give orders the world takes (most seeds drive; a net that prefers to hold may not), each its own way, and a seed plays the same match twice.
- `net_decisions_a_second` (line 579): Decisions a second (phase 0, LR 96): the senses and the net's forward pass for one tank in a world of sixteen, on one thread.

## rotation.rs

Rotation (Paul, 17 October: "every bot should have a push point, and a defense point. the tank does not wait around ... alternate between the adjustable push point and defense point (usually your fortress or base)"):  ```text base (defense point) ----------- push ----------- target D  <-- dwell, resupply    P  <-- dwell, look ---- out ----> <--- home ---- ```  Where a tank used to wait (on guard with nothing in the leash, the squad home and nothing to fetch, the castle standing) it rotates: out to the push point, a look round, home to the defense point, a look round, out again. A fight anywhere on the way is the caller's, as before; the rotation is only what the idle time is spent on.  The push point is `PUSH_FRAC` of the way from the base to the side's objective: the nearest depot not of the side, else the nearest pillbox not of the side, else the map's middle. A fleet shares a base, so its members work out the same point: the fleet's push point. A tank can be given its own (`own`), which is used instead.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 82 | `objective` | Rotation | yes | The side's objective seen from `base`: the nearest depot not of the side, else the nearest pillbox not of the side, else the map's middle. |
| 96 | `fleet_push` | Rotation | yes | The fleet's push point: `PUSH_FRAC` of the way to the objective, then `front_w` of the way toward the front sector nearest that. |
| 107 | `push` | Rotation | yes | This tank's push point: its own, else the fleet's (on ground it can drive to, once `fit` has seen the map). |
| 119 | `peek` | Rotation | yes | The push point as `push` would give it, without changing anything (for the window's coordination screen). |
| 131 | `wing_push` | Rotation | yes | A wingman's push point: from the parked leader toward the side's objective (or toward its own point), `WING_REACH` out at most, pulled back toward the leader onto ground (`ground`). |
| 153 | `fit` | Rotation | yes | With the map: the push point is the tile nearest it that the tank can drive to from the base, without a boat. |
| 191 | `leg` | Rotation | yes | The leg to drive: out to `push` or home to `defense`, turning at each end after `DWELL` (and at home, once refilled). |

Tests (3):

- `the_tank_rotates_between_its_points` (line 224): Out, a look, home, a look, out: the tank never stops for long.
- `the_push_point_leans_to_the_front` (line 268): The push point leans `front_w` of the way toward the front sector nearest it (LR 61); with no front, or at 0, it is as before.
- `a_wingman_pushes_inside_the_ring` (line 288): A wingman's push point: toward the objective, inside the leader's gathering ring, and back toward the leader off the water.

## scenario.rs

Scenario files (roadmap backlog 24, CI-001 VII.B, LAB-REPORT-BRAIN.md section 48; Paul, 18 October: "is there a way to create description and try to attempt these via playtests faster"). A playtest written as text and run headless: the world, the seats that think and how, what is placed and what happens when, and what must hold at the end. Every file under `scenarios/` is a test (`every_scenario_holds`); `make scenario S=file` runs one and prints its trace.  The form is a small part of TOML, read here so the brain takes no new crate: `key = value` lines, `[expect]`, and `[[brain]]`, `[[pillbox]]`, `[[depot]]`, `[[shell]]` and `[[event]]` tables; values are numbers, "strings", true or false, `[lists]` and `{ inline = tables }`. Positions are an anchor and an offset in tiles: `"seat:0"` (where a seat's tank starts), `"depot:0"` and `"pillbox:0"` (the first placed), and `"centre"` (the map's middle). `depots = false` keeps only the depots placed (LR 62). `board = true` gives each team's Commanders one team board, a Commander's `rungs = [...]` the ladder's rungs on, and a `[[depot]]`'s `at` and `offset` move it there (LR 58). A family (`scenarios/families/`) has `runs` and the `floor` it must hold over them (`make families`, backlog 47). A `[[tank]]`'s `toward` heads it at a place, and `met_beyond` asks how far from a point a seat first hit another (LR 61). A Commander's `pings = true` calls what it sees and answers calls (a rally), and `outscores` asks that a seat hit another more often than it was hit back (LR 66).  ```toml name = "a bot drives out from under a shell" players = 2 seconds = 4 [[brain]] seat = 0 kind = "rulebot" [[shell]] owner = 1 at = "seat:0" from = [30, 0] lands_in = 3 [expect] hits_on = { seat = 0, max = 0 } moved = { seat = 0, more_than = 2.5 } ```

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 61 | `num` | Value | yes | A number, or None. |
| 65 | `str` | Value | yes | A string, or None. |
| 69 | `table` | Value | yes | An inline table, or None. |
| 85 | `parse` |  | yes | Read the TOML subset; an error names its line. |
| 125 | `strip_comment` |  |  | A line without its `#` comment (a `#` inside a string stays). |
| 138 | `value_of` |  |  | One value from the front of `s`, and what is left after it. |
| 212 | `snap` |  | yes | A frame of the world as it stands (the clips, LR 102). |
| 232 | `round` | Recording | yes | A recording of `half` tiles each way round `centre`, the map's tiles there kept once. |
| 297 | `run` |  | yes | Run a scenario: the outcome, and every expectation that failed (none is a pass). |
| 306 | `run_seeded` |  | yes | As `run`, on a seed: the world's (`Scenario::seed`, so its map outside what the file carves) and every brain's (theirs xor it), so a run over many seeds samples the scenario's chances (`many`). |
| 313 | `run_net` |  | yes | As `run_seeded`, with a net for each `kind = "net"` brain that names no `file` (the trainer's, LR 99). |
| 319 | `run_with` |  | yes | As `run_net`, with `make` giving the brain for each `kind = "net"` seat instead (a probe of the net's actions, LR 100). |
| 325 | `run_record` |  | yes | As `run_net`, recording a frame every `every` ticks round the learner's start (LR 102, the clips). |
| 332 | `run_inner` |  |  | Every run's body: `run_with`'s, recording into `rec` when given. |
| 797 | `offset` |  |  | An offset in tiles, `[x, y]`; none is no offset. |
| 806 | `check` |  |  | Every expectation that failed, in words. |
| 973 | `reaction` |  | yes | The reaction a file measures (`reaction = "fire"`, `"lob"`, `"move"`, or `"launch:interceptor"`), for the learner, seconds; none when it never came (LR 102, the gauntlet). |
| 998 | `reward` |  | yes | What a training episode earned (LR 99): the sum of the file's `[reward]` terms for the learner's seat (`learner`, 0). |
| 1027 | `many` |  | yes | A scenario over `runs` seeds (`first`, `first + 1`, ...): how many held, the share, and its standard error, sqrt(p (1 - p) / n). |
| 1045 | `set_weights` |  | yes | `SCENARIO_SET="name=value ..."`: those weights set on every brain in the file that holds them (a RuleBot, only the landfall's three), added to its `weights` or a new one, so two settings run on the same seeds. |
| 1066 | `find` |  | yes | A path as given, or from the top of the tree when it is not found there (`cargo test` runs in the crate's folder). |
| 1075 | `run_file` |  | yes | Read and run a file: its failures, or the error that stopped it. |
| 1085 | `dir` |  |  | The scenarios, at the top of the tree. |

Tests (6):

- `a_weight_is_set_on_every_brain` (line 1094): The TOML subset: strings with a `#`, numbers, lists, inline tables, `[[tables]]`, `[expect]`, and errors that name the line.
- `the_subset_reads_values_tables_and_lists` (line 1105)
- `every_scenario_holds` (line 1117): Every file under `scenarios/` holds.
- `scenario_many` (line 1137): One file over many seeds (`RUNS`, 100, from `SEED0`, 1), with `SCENARIO_SET` applied: how often it holds.
- `every_family_holds_its_floor` (line 1158): Every family (`scenarios/families/`, backlog 47) over its `runs` seeds (200) from 1, with `SCENARIO_SET` applied: each must hold at least its `floor` (a share, 0 to 1; none is only reported).
- `scenario_one` (line 1189): One file, with its trace: `SCENARIO=path cargo test -p treadcore-brain --release -- --ignored scenario_one --nocapture` (`make scenario S=path`).

## senses.rs

The senses (LR 95, 96): what a learned tank feels, as about a hundred numbers from 0 to 1, the same on every map and every size of map.  A compass of eight sectors round the tank, relative to its heading (sector 0 ahead; with the map's y growing down the screen, sector k turns k eighths of a turn clockwise: 2 is right, 4 behind, 6 left), and in each sector eleven channels of signal strength: the strongest thing of that kind there, `1 / (1 + d / 8)` for one d tiles off. Then the tank's body; its instruments (where it is on the map, which way it faces, the nearest edge, a hit just taken); a periscope's contacts (LR 97: the nearest enemy, thing to take, depot of ours and friend's call, each as an exact bearing and a strength, the enemy with its closing speed and which way it points, as a submarine's plot reads a contact); and the scripted brain's conclusions as hints. Nothing here is the world's raw lists: a net that learns from these learns what a tank in the field could feel. Every number is 0 to 1 but the contacts' bearings, a sine and a cosine, -1 to 1.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 141 | `dot` |  |  | The dot product. |
| 183 | `bearing` |  |  | A periscope bearing: the sine and cosine of `to`'s bearing from the tank's heading, and its strength. |
| 212 | `strength` |  | yes | A signal's strength from its distance. |
| 217 | `sector` |  | yes | The sector a bearing falls in, relative to `heading`. |
| 223 | `sector_heading` |  | yes | The heading at the middle of a sector. |
| 228 | `at` |  | yes | The index of a sector's channel in the senses. |
| 234 | `sense` |  | yes | Everything the tank feels this tick, or None when it has no tank. |
| 378 | `world` |  |  | Two tanks on a generated map, no pillbox, depot or swarm. |

Tests (7):

- `a_tank_to_the_right_is_felt_to_the_right` (line 389): The compass: a hostile tank set due right of a tank facing any way lights the right sector's hostile channel at its strength, and nothing else lights.
- `the_body_reads_as_shares` (line 414): The body: armour, shells and reload read as shares; a dead tank feels nothing.
- `the_ground_ray_finds_a_wall` (line 430): The ground ray: open ground ahead reads 0; a wall 4 tiles ahead (its tile begins 3 to 4 tiles off, the ray's last open step half a tile short of it) reads between 1 - 3.5 / 16 and 1 - 2.5 / 16.
- `the_periscope_reads_an_enemy_exactly` (line 453): The periscope: an enemy dead right of the tank reads bearing sine 1 and cosine 0 at its strength, in reach at 6 tiles; driving at the tank it closes, pointing at it its aspect is 1; turned away, 0.
- `a_friends_call_is_felt_where_it_points` (line 478): A friend's call: felt in its sector's Call channel and as the call contact, with its kind.
- `the_shell_contact_reads_where_and_when` (line 494): The shell contact (LR 100): a shell landing 4 tiles dead ahead in 1 s reads sine 0, cosine 1, nearness 1 - 4/16, soon 0.75, and outside the splash; nothing for a shell of a friend.
- `the_air_contact_reads_a_strike_and_the_bubble` (line 514): The air contact (LR 100): a hostile strike drone 4 tiles to the right reads sine 1, cosine 0, nearness 1 - 4/16, a threat; the tank's own interceptors read as its bubble, a share of four.

## status.rs

Status as codes (roadmap step 12b, finding L3 of LAB-REPORT-BRAIN.md, section 22). What a tank is doing was only ever a sentence, and the brain's own logic and the census read the sentence back: a guard was idle if its words began "on guard", a focus call went out if the reason said "called", the census counted "ducking" by substring. A rewording broke them silently. Now each sentence is written with a code beside it, and logic and measurement read the code; the words are for people, the code for the program.  The codes are `Copy` enums, set where the words are written (`Commander::set_doing`, `Hop::say`, the cycle's `act`). `code()` is a stable dotted name for logs, the census and the ledger.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 33 | `word` | KiteStep | yes | The words, as the status line has always had them. |
| 47 | `code` | KiteStep | yes | The step's code, the last part of `kite.<code>`. |
| 61 | `stepping_in` | KiteStep | yes | A step toward the target: closing or stepping in. |
| 98 | `code` | HopAct | yes | The step's code, the last part of `hop.<code>`. |
| 142 | `word` | CycleAct | yes | The words of an improvement (`Cycle::improvement`). |
| 155 | `code` | CycleAct | yes | The stage's code, the last part of `cycle.<code>`. |
| 234 | `code` | Act | yes | A stable dotted name: `kite.step_in.reloading`, `hop.bridging`. |
| 246 | `plain` | Act |  | The code of an act with nothing inside it. |
| 298 | `kite` | Act | yes | The kiting step in it, if any. |

Tests (1):

- `every_code_is_its_own` (line 345)

## tactics.rs

Fleet tactics (Paul, 16 October: "we want a bunch of scalars and reasons to collaborative bot behavior. research ... fleet tactics in rts"). Three decisions a team's bots made by distance alone, each now a score of weighted considerations with the reasons kept for the status line, the log and team chat:  1. Which hostile tank to shoot (target selection). The RTS rule is focus fire: a side that puts its shots on one tank at a time takes tanks out of the fight sooner, and a tank out of the fight shoots no more (Lanchester's square law, below). So a target scores for being near, wounded, one shot from dead, afloat (a boat sinks at one hit), called by a teammate's ping, already under a teammate's gun, in range now, and threatening a teammate; and it scores less once more of us are on it than it takes (overkill: the rest pick the next). Uriarte and Ontañón score StarCraft targets the same way, by distance, threat and damage; the GA micro bots by health alone. 2. Whether to close (the engagement). Lanchester's square law: with aimed fire, a side's strength is its number squared times its quality, so three tanks beat two by more than three to two, and a side that arrives one at a time is beaten one at a time. The odds at a fight are ours squared over theirs squared; below `engage_odds` with teammates on the way a bot holds off and masses with them ("don't trickle in"), above `press_odds` it closes. 3. Who answers a call (assignment). A ping drew every bot in reach, the base's too. Now the first `answer_cap` teammates nearer to it than this bot are enough, and a crew on a job answers only inside `crew_reach`.  And where to stand (the path's end): of the flank's firing points round a target, the one least exposed to its friends' guns and to hostile pillboxes, as an influence map would pick it, without one.  And kiting (Paul, 17 October: "more directed kiting, where we estimate the range of the opponent and avoid getting hit while aggressing closer in a group"). Every tank's direct gun reaches the same 10 tiles and reloads in 4 s, and a tank's reload is in the observation (a window shows it), so the estimate is the target's reach (its direct gun while it has shells), when it can fire again, and which other guns would cover the spot a step in ends on. The bot waits on the ring just past that reach, where its own indirect gun still works (12.5 tiles); it steps in to 9 tiles, fires direct and steps out again only while the target's gun is reloading for longer than the trip takes, no other loaded gun covers the spot, and its own gun is loaded. Pressing (the square law in our favour), it also steps in while a teammate is already inside the target's reach: the target has one gun and many of us. Massing, it never does.  The considerations add; each is a weight times a value in 0..1, so a weight reads as "worth this many of the others". `Tactics::default` is the team's; a test or a bot of another temper changes the numbers.

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 137 | `default` | Tactics |  | *(the `Default` trait's)* |
| 191 | `ratio` | Odds | yes | Lanchester's square law: ours squared over theirs squared. |
| 220 | `reach` |  | yes | How far a hostile tank's gun reaches now: its direct gun while it has shells. |
| 229 | `ready_in` |  | yes | Seconds until a tank can fire again. |
| 281 | `worth` |  |  | A tank's worth in a fight: armour, and none without shells. |
| 291 | `with` | Tactician | yes | A Tactician of these weights, with nothing heard yet. |
| 296 | `hear` | Tactician | yes | Every tick: a teammate's ping is a call on what is under it. |
| 309 | `score` | Tactician | yes | A target's score and the reasons that made it. |
| 317 | `value` | Tactician | yes | The score alone: what `pick` asks of every candidate, with no words built for the ones that lose (roadmap 12b). |
| 323 | `rate` | Tactician |  | A target's score; the reasons pushed to `why` when it is Some, and their tags set in `tags` (`score` wants them, `value` not). |
| 376 | `pick` | Tactician | yes | The hostile tank to shoot, of those `fair` allows; the reason kept. |
| 428 | `odds` | Tactician | yes | The odds at a fight round `at`, me counted as there. |
| 452 | `stance` | Tactician | yes | Mass, engage or press, by the odds; massing ends after `mass_for` seconds whether or not the team came. |
| 507 | `kite` | Tactician | yes | Kiting `target`, coming in on `from`, at `stance`: the step in when the target's gun cannot answer it, else the waiting ring (module notes). |
| 553 | `enough_answer` | Tactician | yes | Teammates nearer a ping than me: past `answer_cap` I stay. |
| 563 | `stand` | Tactician | yes | Where to stand against `target` at `range`, coming in on `from` (a unit bearing from the target): of seven points on the arc within 60 degrees of it, the one with the fewest other guns on it, the bearing breaking ties. |

Tests (3):

- `focus_fire_picks_the_wounded_tank_a_teammate_is_on` (line 591): Two tanks with a pillbox-free field: a fresh near one, a far wounded one an ally is already shooting.
- `kiting_steps_in_only_when_the_gun_cannot_answer` (line 624): Kiting's forks, one at a time: the target loaded holds us past its reach; reloading lets us step in; a second loaded gun on the spot holds us again; our own gun not loaded by the time we would be in holds us; massing...
- `the_square_law` (line 679)

## weights.rs

Weights and capabilities (roadmap step 12c, LAB-REPORT-BRAIN.md sections 4 and 5). Every number a tuner may touch is one slot of a flat array, `Weights`, named by `W`; every part of the brain that can be switched off is one bit of `Caps`. The array is lent to whatever scores (`&Weights`), never cloned per think, and written whole into the ledger (ledger.rs) beside the match it played, so a high score can be played again and bred from.  The defaults are the constants the brain had on 17 October: the 22 weights of `Tactics` and its other settings as they were, and the new parts' numbers as the lab report starts them. A capability that is new is off by default, so the brain of 17 October is the baseline until a census promotes a part (ROADMAP goal 4).  The file form is TOML's simplest subset, a `name = value` a line and `#` comments, so a person can write one by hand and `--weights FILE` reads it without a parser crate:  ```text # bolder w_wounded = 1.5 press_odds = 1.2 caps = smooth, forget, priced_crossing ```

| Line | Function | Of | Pub | What it does (its doc comment) |
| --- | --- | --- | --- | --- |
| 38 | `name` |  | yes | The weight's name in a weights file and on a slider. |
| 42 | `default_value` |  | yes | The weight's default, the brain's as shipped. |
| 46 | `doc` |  | yes | The weight's doc line: what it does, and why its default. |
| 50 | `by_name` |  | yes | The weight of that name, if any. |
| 56 | `step` |  | yes | A slider's step (the coordination screen, backlog 15): one for a rounded weight, else a tenth of the default taken to 1, 2 or 5 of its power of ten; a tenth for nothing. |
| 207 | `default` | Weights |  | *(the `Default` trait's)* |
| 214 | `index` | Weights |  | *(the `Index` trait's)* |
| 220 | `index_mut` | Weights |  | *(the `IndexMut` trait's)* |
| 239 | `doc` | Weights | yes | What a capability does, for the sliders' page. |
| 268 | `default` | Caps |  | *(the `Default` trait's)* |
| 275 | `has` | Caps | yes | Every bit of `c` is on. |
| 279 | `with` | Caps | yes | These, and `c` on. |
| 283 | `without` | Caps | yes | These, and `c` off. |
| 287 | `all` | Caps | yes | Every capability, the new ones too. |
| 291 | `by_name` | Caps | yes | The capability of that name, as a weights file spells it. |
| 295 | `names` | Caps | yes | The names of the bits that are on, comma-separated. |
| 308 | `fmt` | WeightsError |  | *(the `Display` trait's)* |
| 324 | `parse` | Settings | yes | Read the file form. |
| 364 | `smooth` | Settings | yes | How much the bot smooths (`w_smooth`), 0 to 1. |
| 370 | `smooth_alpha` | Settings | yes | The share of a move's new pull taken a think: all of it at `w_smooth` 0, `alpha` at 1, in a line between. |
| 375 | `smooth_on` | Settings | yes | The yes-or-no parts of smoothing: on above 0.5. |
| 380 | `to_text` | Settings | yes | The file form: every weight, and the capabilities. |
| 389 | `diff` | Settings | yes | The weights that differ from the defaults: what makes a genome. |
| 395 | `nudge` | Settings | yes | A slider moved `by` steps: never under nothing, and kept to four places so a step back is the number it was. |
| 401 | `ticks` | Settings | yes | Seconds into ticks, for the time weights. |
| 408 | `from_weights` | Tactics | yes | The tactics the weights describe. |
| 464 | `eval` | Curve | yes | The curve at `x`, clamped to 0..1 both ways; a value that is not finite reads 0. |
| 484 | `utility` |  | yes | An action's utility (LR 4): its weight times the geometric mean of its considerations, raised by `commit` if it is the intent held now. |

Tests (6):

- `default_weights_are_todays_constants` (line 505)
- `a_weights_file_round_trips_and_names_its_mistakes` (line 516)
- `smooth_is_a_weight_that_blends_and_thresholds` (line 536): `w_smooth` (LR 46): the default is near off, the desire vector blends in a line, and the yes-or-no parts turn on above 0.5.
- `the_geometric_mean_compensates_and_vetoes` (line 552)
- `a_slider_steps_and_steps_back` (line 562)
- `every_profile_reads_and_differs` (line 578)
