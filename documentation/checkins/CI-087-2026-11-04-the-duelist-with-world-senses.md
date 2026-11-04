# The Duelist with World Senses: the Gladiator, Style 17, and the Bridge That Reads Each New World Champion

**Check-in report CI-087.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 138; CHANGELOG; ROADMAP
backlog 80; journal 0114; `nets/styles.tsv`; in orrery, the bridge stage
(`tools/plans/treadcore.py` `bridge_jobs`, `tools/bridge.py`) and the
world build of crucible (`src/crucible.cu`, WORLD).

---

## Abstract

crucible's GPU islands have trained under the game's rules since the
evening of 3 November, with 55 inputs and two hidden layers of 48. The
game's bridge to crucible (style 7, the Duelist, LR 126) read only the
old arena's 35-input hybrid. The bridge now reads a world champion with
its twenty world inputs built from what a tank in the game sees, as
crucible builds them, and drives as crucible's world tank: a heading
error, no reverse. It ships as a new style, 17, the Gladiator (world
milestone M0015), beside the hybrid Duelist. On the same 48 seeds the
Gladiator reads +4.99 ± 2.08 a seat over the Duelist on every map (+11.56 ±
1.75 on the generated maps). The fleet reads a champion not yet shipped
by `GLADIATOR=file`, and orrery's bridge reads each new world milestone
against the shipped one; shipping it is a commit, pressed.

## I. Intent

Paul, of orrery's question that morning ("You wrote: 'the GPU build
always looks like the perfect world in treadcore. Incorporate GPU as a
growth step in the pipeline.' Which do you mean?"), chose all three:
crucible follows treadcore, champions enter the line, the GPU trainer as
a stage. The first was built in orrery and crucible. For the second,
orrery said what it would cost (treadcore's Duelist taught 20 world
senses and the larger net, a new champion shipped in the game, a commit
that changes how a style plays) and asked. Paul, word for word:

> yes do step 2, the duelist with world senses

## II. Decision log

| Decision | Cause |
| --- | --- |
| A world champion is read by its size (5,236 floats), a hybrid by its (2,340); a plain build's (no gate) is refused | crucible writes champions as bare floats; a Duelist's net must carry the gate (section 126), or it takes every fight |
| The twenty inputs built as crucible.cu's `sense` under WORLD builds them, index for index | a champion hears only what it was trained on; any other order is noise to it |
| The eyes end at `TANK_VISION` for the rays; the bearing and the enemy's armour stay | crucible does the same: its rays stop at VISION, its 28th to 30th inputs do not |
| A pillbox "that would fire on it" is a live neutral one or an enemy's | crucible's pillboxes are neutral and fire on either tank; in the game a side's own fires on nobody of it |
| The incoming shell is the nearest hostile direct shell where it is now along its flight | crucible's 31st input is the enemy shell's position, not its landing point |
| The turn is a heading error of up to a half turn, the throttle has no reverse (under a twentieth: `Stop`) | crucible's world driver, which is treadcore's driver |
| The gun stays the rules' | section 126: the champion's fire output means its hull is on the enemy |
| A new slot, 17, not style 7's file replaced | "a slot once given keeps its meaning" (styles.rs); and one binary then reads both on the same seeds |
| `nets/gladiator.bin` is M0015 | the world ladder's newest milestone that morning: judged better than every one before it by crucible's ladder |
| `GLADIATOR=file` for the fleet's reads | the champion is in the binary (SHIPPED, NOT LOCATED), and a read must not wait for a commit; it is the hook `CENSUS_WEIGHTS` is for dials |
| Shipping is a press, not automatic | CLAUDE.md: commit only when Paul asks; a shipped champion changes how style 17 plays |
| The rule written before the run | CLAUDE.md, "the rule before the run" |

## III. Method

Code in `source/treadcore-brain/src/styles.rs`: `Champion { nin, h, w }`
and `Champion::world`; `world_senses`, `scripted_world`, `shell_at`; the
world branch of `Duelist::duel`; slot 17 in `ROSTER`; `gladiator_file`.
`weights.rs`: `bot_style`'s choices end in `gladiator`. Seven tests new
(the sizes, the arithmetic at 55-48-48-4, the world yardstick, eyes past
`TANK_VISION`, a pillbox that has the tank and one of the side's own,
an open gate with no throttle holding the tank). The brain crate's 178
tests pass.

The rule, before either census: the Gladiator is better than the
Duelist if, on the same 48 seeds, its read against the defaults is
higher by more than two standard errors of the paired difference.

Censuses: `scripts/census.sh` on this Mac, `JOBS=8`, `SEEDS=48`,
`SEED0=0`, `SECS=900`, `RESPAWNS=3`, `TRIAL` a file of one line
(`bot_style = 17`, then `bot_style = 7`), the defaults as the base. The
pair is read seed by seed from each job's `a seed:` list (the jobs split
the seeds alike). The first pair's `MAP=all` played the generated map
only: BSD `paste` needs a `-`, which orrery's census jobs supply and a
census by hand on a Mac does not. The second pair ran with that shim and
played every map.

## IV. Results

| Read, a seat against the defaults | generated maps | every map |
| --- | --- | --- |
| Gladiator (style 17, world M0015) | −2.25 ± 0.99 | −2.53 ± 1.21 |
| Duelist (style 7, hybrid) | −13.80 ± 1.35 | −7.52 ± 1.60 |
| Gladiator less Duelist, paired | **+11.56 ± 1.75** | **+4.99 ± 2.08** |

The rule is met on both pairs: 6.6 errors on the generated maps, 2.4 on every map. By map (every map, 4 or 5 seeds each, inside the noise one by one): island +3.16 against −15.95, big-lizard +2.59 against −12.26, industry −1.10 against −11.66, atlantic −3.57 against −12.25; the other way, fortressisles2 −8.93 against +3.43 and europe −7.83 against +3.12.

## V. Discussion

A champion of the toy arena, read in the game, cost its team 7.5 a seat
on every map (13.8 on the generated maps); a champion of the world
arena, read through the same gate, costs it 2.5 (2.3). The world islands
trained against pillboxes and inside the game's own reach and speed, and
the bridge now tells it the same things: that is the whole of the
change, and the read says it was most of the gap. What remains is the
Gladiator's own gap to the rules (it still reads under the defaults),
which is the islands' to close and the bridge's to measure each time a
milestone is taken.

## VI. What is not built

crucible's own `sense` run on a game state, for parity to the decimal
(it runs only on a card); the bridge's first reads on the fleet (they
need this commit on main); the GPU behaviour-cloning stage (Paul's step
3), its net and its judge not yet chosen.

## VII. Paul's decisions

Whether to commit and push this (CLAUDE.md); whether a candidate that
reads better should ship by itself in auto mode, as a promotion does,
or stay a press; which net the GPU behaviour-cloning stage trains and
how it is judged.
