# Training Inside the GPU: a Self-Contained Learning Loop on Three Pascal Cards

**Lab report.** Paul Richeson; Claude Code (Claude Opus 5.5) as the pair.
28 October 2026. Project `crucible` (`src/crucible.cu`, 543 lines).
Companion records: treadcore `LAB-REPORT-BRAIN.md` section 121, CI-080.

---

## Abstract

The fleet has three NVIDIA Pascal cards: a GTX 1080 Ti (calber), a GTX
1080 (whitewolf) and a GTX 1070 Ti (boburner). We asked whether a
self-learning AI, with sensors and its own loop, could run entirely on
one card. It can.

crucible keeps all of the following on the GPU:
- the world, a two-tank arena with pillars;
- the sensors, twelve rays per tank;
- the brain, a 32-32-32-3 net per individual;
- the play, every individual's duels at once;
- the selection and the mutation;
- the memory, a hall of fame of 4,096 past champions held on the card.

The host only launches kernels and reads back two numbers a generation.

**Throughput.** One block per individual and one thread per duel, with
both nets in shared memory, plays:

| Card | Tank-steps per second | Net arithmetic |
| --- | --- | --- |
| GTX 1080 Ti | 581 M | 2.5 TFLOP/s |
| GTX 1080 | 462 M | 2.0 TFLOP/s |
| GTX 1070 Ti | 419 M | 1.8 TFLOP/s |
| **Fleet** | **1.46 G** | **6.3 TFLOP/s** |

**Registers.** Limiting each thread to 128 registers doubled the
throughput (190 registers gave 236 M tank-steps a second on the 1080).
Tighter limits spill and are slower.

**Learning.** The first loop, self-play against the hall of fame alone,
did not learn to beat a scripted yardstick: it won 8.2 % ± 2.7 % of games
after 300 generations. With half of each generation's games played
against sparring bots of drawn skill:
- whitewolf: 21.9 % ± 3.0 %, 3.4 standard errors better, confirming the
  rule written before the run;
- calber, a replicate with another seed: 14.3 % ± 2.1 %, 1.8 standard
  errors better, short of the bar.

**Memory and power.**
- Memory is not the limit. The run used 313 MB of 8 GB; a population
  of about 400,000 such nets would fit.
- The limits are shared memory per block (it caps the hidden layer at
  48) and the work per thread.
- Under load the cards drew 55 % (1080) to 66 % (1080 Ti) of their
  power limits, so there is headroom left. Section VII says where.

---

## I. Introduction

### A. The prompt

> we need to consider the interface from treadcore . the styling. thr rows and columns of brushed metal. Can we redesign all tabs all sub tabs and rethink about the interface progression. tell a story, help us orchestrate and add comptuers to configure in the orchestration. For example, we want to create a second project to run in cuda. Perhaps we can size something for the 8GB cards we have. Is there a neural net that we can train and retrain to keep its generations within itself. We need sensor data and a self contained loop is there a way to create a different style of self learning ai on these cards. specifically targeted for CUDA program that will stress the cards and we can use them to their maximum, at their fastest. How much of this simulation can be ran fully in cuda itself. If we can do that, can we write a lab report about general purpose traing and scaling up. and inside the gpu. we have an assessment of our fleet, we have 1070ti , 1080, 1080ti.

### B. The questions, in the order answered here

1. Can a self-learning AI, with sensors and a loop of its own, run
   entirely on one of these cards? (Sections III and VI.)
2. What size fits an 8 GB card, and what actually limits it?
   (Section IV.)
3. How fast does each card run it, and how near its maximum?
   (Section VI.)
4. How much of treadcore's own simulation could run in CUDA?
   (Section VII.A.)
5. How does this scale, inside a card and across the fleet?
   (Section VII.B, C.)

The interface redesign in the same prompt is the next step. It is not
in this report.

### C. Why this step comes first

The interface's story and its orchestration of "a second project to run
in CUDA" need that project to exist, with its real numbers.

---

## II. The Fleet's Cards

Read on 28 October with `nvidia-smi` and `cudaGetDeviceProperties`.
All three run driver 580.178.04 and CUDA 12.9, compute capability 6.1
(`sm_61`).

| | GTX 1080 Ti | GTX 1080 | GTX 1070 Ti |
| --- | --- | --- | --- |
| machine | calber (.181) | whitewolf (.124) | boburner (.113) |
| SMs (x128 FP32 lanes) | 28 (3,584) | 20 (2,560) | 19 (2,432) |
| memory | 11 GB, 484 GB/s | 8 GB, 320 GB/s | 8 GB, 256 GB/s |
| power limit | 250 W | 215 W | 180 W |
| FP32 peak at spec boost | 11.3 TFLOP/s | 8.9 TFLOP/s | 8.2 TFLOP/s |
| SM clock under this load | 1,917 MHz | 1,952 MHz | not logged |
| host | 8 threads, 62 GB | i7-6700K, 62 GB | Ryzen 7 3700X, 31 GB |

What Pascal does and does not have, and what each fact means here:
- **No tensor cores.** Matrix work is plain FP32 fused multiply-adds.
- **FP16 at 1/64 of the FP32 rate on these chips.** Half precision is
  slower, not faster. FP32 is the type.
- **INT8 `dp4a`:** four 8-bit products and a sum in one instruction.
  It is the one path to more arithmetic for a quantized net (VII.B).
- **48 KB of shared memory per block, 96 KB per SM, 64 K registers per
  SM.** These set the sizes in Section IV.
- **CUDA 13 drops Pascal, and the 580 driver branch is the last for
  it.** Keep these machines on CUDA 12.9 and driver 580: do not let an
  upgrade take them to 13.

---

## III. Design

### A. The loop, all of it on the card

```
          ┌──────────── one generation, all kernels on the GPU ────────────┐
          │                                                                │
population│  play<<<P blocks, EPS threads>>>                               │
 P nets ──┼─▶ block i: net i + an opponent net in shared memory            │
          │   thread k: one duel, 600 steps:                               │
          │     sense (12 rays, bearing, own state) → think (the net)      │
          │     → drive, fire → shells fly → hits                          │
          │   score: damage dealt − taken ± 2 for a win or loss, + aim     │
          │                                                                │
          │  thrust::sort_by_key (scores, ranks)                           │
          │  choose: elite 16 kept; others' parents from the top quarter   │
          │  breed: every weight of every child = parent's + σ·N(0,1)      │
          │  every 5th generation: champion → hall of fame (a ring of 4096)│
          └──────────────────────┬─────────────────────────────────────────┘
                                 │ to the host: best and mean score (8 bytes)
                                 ▼
                every 10th generation: the champion vs the yardstick, 4096 duels
```

**Why evolution, not gradient descent.** The world is not
differentiable: shells hit or miss, and tanks bump into pillars. A
gradient would need a model of the world or a policy-gradient estimator
with its replay buffers on the host. Evolution needs only a score per
individual, and the individuals are independent. Every SM stays busy
with no communication until the sort.

### B. The world and the sensors

| Part | What it is |
| --- | --- |
| Arena | 64 × 64 units; six pillars of radius 1.5 to 3.5, new for every duel |
| Tank | radius 1, 3 hit points |
| Speed and turning | at most 6 units a second; 2.5 radians a second |
| Shell | 30 units a second, 1.2 second life; one in flight per tank; 1 second reload |
| Step | a tenth of a second; a duel is at most 600 steps (60 s) |

Each tank's sensors are 32 numbers:
- **Twelve rays round the tank.** Each gives two numbers:
  - the nearest thing it meets (a wall, a pillar or the enemy), as
    1 − distance / 40;
  - the enemy, if the enemy is the nearest thing it meets.
- **Its own state:** its speed, its hit points, and whether it is
  loaded.
- **The enemy:** its hit points, and the sine and cosine of the bearing
  to it.
- **Danger:** how near the enemy's shell is, within 10 units.
- **A constant 1.**

### C. The brain

A net per individual: 32 inputs, two hidden layers of H = 32 (tanh),
and three outputs (turn, throttle, fire if > 0). That is 2,211 weights,
8.6 KB.

### D. The mapping, and why it stresses the arithmetic, not the memory

```cpp
__global__ void __launch_bounds__(256, MINB) play(const float* pop, const float* hall, ...) {
    extern __shared__ float sm[];
    float* wA = sm;          // this block's individual
    float* wB = sm + NW;     // its opponent, from the hall of fame
    for (int i = threadIdx.x; i < NW; i += blockDim.x) { wA[i] = pop[ind * NW + i]; wB[i] = hall[opp * NW + i]; }
    __syncthreads();
    // ... one duel per thread; in think(), every thread of a warp reads
    // the same weight W[j*NIN+i] at the same instant: a shared-memory
    // broadcast, no bank conflicts, and the state stays in registers.
}
```

The weights are read once from device memory for every 128 to 256
duels × 600 steps. After that, the work is fused multiply-adds from
shared memory into registers. The DRAM bandwidth (256 to 484 GB/s)
never enters.

### E. Randomness

All randomness is counter-based (a 32-bit integer hash of seed,
generation, block, thread and weight index). There is no RNG state to
store or advance. Any thread can draw the noise for any weight, and the
same seed replays the same run.

### F. "Keep its generations within itself"

- **The hall of fame.** Every fifth generation's champion is copied,
  device to device, into a ring of 4,096 slots: 20,480 generations of
  history, 36 MB.
- **What the learner plays.** Half its games against the hall are with
  one of the newest 16 champions, half with any of them. It keeps
  meeting its own past and so cannot forget how to beat it.
- **`--save` and `--load`.** They write and read the population and
  the hall as one file (54 MB at P = 2,048). Training resumes where it
  stopped: the retraining asked for.

---

## IV. Sizing for an 8 GB Card

With NW weights per net, population P and hall capacity CAP:

```
device memory  = 4 · NW · (2P + CAP)  bytes      (two population buffers: this generation, the next)
shared / block = 8 · NW               bytes      (two nets)
```

| Net | NW | 2 nets in shared memory | P on 8 GB (CAP = 4096) |
| --- | --- | --- | --- |
| 32-32-32-3 | 2,211 | 17.7 KB | ~420,000 |
| 32-48-48-3 | 4,083 | 32.7 KB | ~225,000 |
| 32-64-64-3 | 6,467 | 51.7 KB > 48 KB: **does not launch** (measured: "invalid argument") | — |

So on these cards the population is limited by time, not memory. The
net's width is limited by shared memory, unless the opponent's weights
are read from L2 or the net is split across a warp (VII.B).

What uses the 8 GB in practice is **many independent runs at once**. A
run at P = 2,048 takes 313 MB, so twenty fit on one card. That is the
natural form for a fleet: one run per seed or per setting, as the
treadcore queue already runs its censuses.

---

## V. Methods

### A. The build

On each GPU node (`~/code/crucible`, copied from the Mac):

```
make            # nvcc -O3 -use_fast_math -std=c++17 -gencode arch=compute_61,code=sm_61 -DHID=32 -DMINB=2
```

### B. Throughput

```
./crucible --pop 2048 --gens 4 --bench
```

Only the play kernel runs: 256 duels per block, 600 steps, both tanks
nets. The middle three generations are reported.
- **Register limits:** whitewolf, with MINB = 1, 2, 3 and 4, giving
  190, 128, 80 and 64 registers per thread.
- **Width:** H = 48 and 64 at MINB = 2.
- **Across the fleet:** calber and boburner at H = 32, MINB = 2.

### C. Learning

**The runs:**

```
./crucible --pop 2048 --eps 128 --gens 300 --yard 10 --seed 7 --spar 0      # whitewolf, before sparring existed
./crucible --pop 2048 --eps 128 --gens 400 --yard 10 --seed 7               # whitewolf, sparring 0.5
./crucible --pop 2048 --eps 128 --gens 400 --yard 10 --seed 8               # calber, sparring 0.5
```

- The first run predates the `--spar` flag. It is the same loop with
  every opponent from the hall.
- **Shared settings:** σ = 0.03, elite 16, the hall fed every 5th
  generation.
- **Logged:** `nvidia-smi` every 2 s (utilisation, power, temperature,
  SM clock, memory).

**The yardstick** is a scripted tank. It turns to the enemy, keeps
about 11 units off, and fires when aimed within 0.12 radians. It is
read every 10th generation on 4,096 duels on seeds kept apart from
training (seed XOR 0xA5A5).

**Sparring partners** are the same script with a drawn aim tolerance
(0.05 to 0.6 radians) and range (6 to 20 units). The yardstick's own
skill is inside that range, so the yardstick is held out in seeds, not
in kind.

**The rule** (written before the sparring runs): sparring works if the
champion's yardstick win rate, averaged over the last five reads, is
more than two standard errors above the no-sparring run's last five.
The standard error is that of the five reads; the two arms' errors are
combined in quadrature.

---

## VI. Results

### A. The register limit

GTX 1080, P = 2048, 256 duels a block:

| MINB | registers / thread | blocks / SM | occupancy | Mtank-steps/s | net GFLOP/s |
| --- | --- | --- | --- | --- | --- |
| 1 | 190 | 1 | 12 % | 236 | 1,016 |
| **2** | **128** | **2** | **25 %** | **462** | **1,983** |
| 3 | 80 | 3 | 38 % | 238 | 1,019 |
| 4 | 64 | 4 | 50 % | 185 | 793 |

Occupancy pays only until the compiler must spill a duel's state to
local memory, which happens below 128 registers. Here, the best is
twice as many threads that still hold their state in registers. It is
not the highest occupancy.

### B. The width

GTX 1080, MINB = 2:

| H | weights | Mtank-steps/s | net GFLOP/s |
| --- | --- | --- | --- |
| 32 | 2,211 | 462 | 1,983 |
| 48 | 4,083 | 260 | 2,074 |
| 64 | 6,467 | does not launch (51.7 KB of shared memory) | — |

The net's arithmetic rate barely moves, so a wider net costs steps in
proportion to its size.

### C. The three cards

H = 32, MINB = 2:

| Card | Mtank-steps/s | net GFLOP/s | per SM | of FP32 peak, net only |
| --- | --- | --- | --- | --- |
| GTX 1080 Ti | 581 | 2,494 | 20.8 M | 22 % |
| GTX 1080 | 462 | 1,983 | 23.1 M | 22 % |
| GTX 1070 Ti | 419 | 1,797 | 22.0 M | 22 % |
| **Fleet** | **1,462** | **6,274** | | |

**How the cards compare.** Throughput follows SMs × clock: all three
reach the same 22 % of peak in the net alone. The 1080 Ti's lower rate
per SM is its lower clock under load (1,917 MHz against 1,952 MHz).

**What "net GFLOP/s" counts.** It counts only the net's multiply-adds
(2 × 2,211 per tank-step). The twelve rays, about 100 circle and wall
tests per tank-step, and the physics are real work it leaves out. The
card is doing more than the column says.

### D. Learning

Win, draw and loss rates against the yardstick, read every 10th
generation (shown every 40th):

| generation | 10 | 50 | 90 | 130 | 170 | 210 | 250 | 290 | 330 | 370 | 400 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| whitewolf, no sparring | 0.00 | 0.01 | 0.02 | 0.13 | 0.02 | 0.04 | 0.26 | 0.07 | | | |
| whitewolf, sparring | 0.00 | 0.01 | 0.02 | 0.02 | 0.05 | 0.08 | 0.14 | 0.12 | 0.24 | 0.20 | 0.25 |
| calber, sparring | 0.04 | 0.01 | 0.01 | 0.01 | 0.05 | 0.07 | 0.10 | 0.06 | 0.21 | 0.21 | 0.18 |

The rule, on the last five reads:

| Run | Last five reads | Mean ± se | Against no sparring | Verdict |
| --- | --- | --- | --- | --- |
| no sparring (gens 260–300) | 0.18, 0.02, 0.08, 0.07, 0.06 | 0.082 ± 0.027 | — | — |
| whitewolf, sparring (360–400) | 0.32, 0.20, 0.17, 0.16, 0.25 | 0.219 ± 0.030 | +0.137 ± 0.040, **3.4 se** | confirmed |
| calber, sparring (360–400) | 0.11, 0.21, 0.10, 0.12, 0.18 | 0.143 ± 0.021 | +0.061 ± 0.034, 1.8 se | short of the bar |

**A check at equal age** (generations 260–300 for all three):
- no sparring: 0.082 ± 0.027;
- whitewolf, sparring: 0.210 ± 0.030 (3.2 se);
- calber, sparring: 0.115 ± 0.015 (1.1 se).

The sparring runs were still improving at 400 generations. Neither has
levelled off.

**The score saturates.** The training score's best was near its
ceiling of about 5 (three hits taken for none, plus the win) from the
first generation. Some individual always beats a weak opponent cleanly.
The mean score and the yardstick are the readable signals; the best
score is not.

### E. Power and heat

| Run | Busy samples | Power | Of limit | Max temperature | SM clock | Memory |
| --- | --- | --- | --- | --- | --- | --- |
| GTX 1080, sparring | 108 | 118 W mean, 129 W max | 55 % of 215 W | 77 °C | 1,952 MHz | 313 MiB |
| GTX 1080 Ti, sparring | 82 | 165 W mean, 188 W max | 66 % of 250 W | 68 °C | 1,917 MHz | 345 MiB |

Utilisation read 100 % throughout. That figure only means a kernel was
resident: the power shows the lanes were not all busy.

### F. Time

| Run | Seconds a generation | Total |
| --- | --- | --- |
| whitewolf, sparring (2,048 × 128 duels, 400 generations) | 0.54 | about 3.6 minutes |
| calber, sparring | 0.41 | |

At these rates a card plays about a billion duel-steps a minute.

---

## VII. Discussion

### A. How much of treadcore's simulation could run in CUDA

**treadcore's world.** `treadcore-sim` is 8,437 lines. A step runs ten
phases: tanks, chains, pillboys, drones, pillboxes, shells, depots,
structures, swarm and match. Its state is growable lists (proposals,
events, routes) with strings, chat, alliances and diplomacy.

**treadcore's scripted brain.** It is about 19,800 lines of branching
rules.

**The fit, part by part:**
- **Fits a GPU well:** movement, shells, line-of-sight rays and tile
  collisions on the map. They are the same kind of work crucible does,
  and would run there at similar rates.
- **Fits poorly:**
  - the diplomacy, chat and event lists, which grow, allocate and
    branch;
  - the scripted brain: neighbouring threads would take different
    branches, and a warp runs the union of its threads' paths.
- **Fits as it is now:** treadcore's learned net, the `bc` brain. It
  is a net, so it fits as crucible's does.

So "all of it" is true for a world designed for the card, like
crucible's. For treadcore, the honest answer is the combat core and
the net, not the whole game. The useful split:
- treadcore's census stays on the CPU fleet, where its rules are
  exact;
- a GPU arena built from treadcore's combat rules could pretrain or
  sharpen the net by the billion steps;
- the census remains the judge.

### B. Scaling up inside a card

The cards drew 55 to 66 % of their power, and the net reached 22 % of
peak. In the order the headroom is likely to be:

1. **More independent work per thread.** Two duels per thread
   interleave two dependency chains. The scheduler then has a ready
   instruction while the other chain waits, which is what 25 %
   occupancy lacks.
2. **Fewer transcendentals.**
   - Each tank-step calls `tanhf` 67 times and `__sinf`/`__cosf`
     about 30 times. They run on the special-function units, 32 per SM
     against 128 FMA lanes.
   - A rational tanh, and the ray directions turned by one rotation
     instead of twelve sines and cosines, move that work onto the FMA
     lanes.
3. **A warp shares a net.** For nets wider than 48, lane j computes
   neuron j for all 32 duels of a warp, exchanging inputs by shuffle.
   - The weights can then live in registers spread across the warp,
     not in shared memory, which lifts the 48 KB limit on width.
   - This is the general form of batched inference without tensor
     cores.
4. **INT8.** With weights quantized to 8 bits, `dp4a` gives four
   multiply-adds per instruction. Evolution needs no gradients, so the
   net can be trained in its quantized form. This is a natural
   experiment for these cards.
5. **Launch overhead and the host.** A generation is about 0.5 s of
   play against a few hundred microseconds of sort and breed. The loop
   is already GPU-bound, and CUDA Graphs would buy little here.

### C. Scaling across the fleet: islands

The fleet's three cards are three islands:
- Each runs its own population on its own seed.
- Every N generations each sends its champion, 8.6 KB, to the
  coordinator.
- The coordinator writes the newest champions from the other islands
  into every island's hall of fame (a `--load`-style import).

What this costs and gives:
- **Communication:** kilobytes a minute. The ssh star from the Mac
  carries it with no change.
- **Speed:** each card runs at its own pace, so the fleet's 1.46 G
  tank-steps a second add up with no waiting.
- **Diversity:** different seeds and different opponents keep the
  islands from all converging on one style.

In orrery's queue, `crucible` becomes a run kind for hosts with the
`gpu` cap:
- **A run** is one island's segment: `--load` the state, play K
  generations, `--save` it.
- **The planner's work:** pass the champions between islands, and
  read the yardstick numbers as progress, as it already reads
  treadcore's censuses.

### D. General-purpose training on the GPU: what this run teaches

1. **Put the whole loop on the card,** and read back only what a
   person reads: scores, a win rate. Here a copy back every generation
   would have cost more than the sort.
2. **Map the parallel axis to the independent axis.**
   - Individuals to blocks and episodes to threads, so a block's
     threads share weights and diverge only in their own world.
   - Episodes that end early leave a thread idle. Equal-length
     episodes, or packing finished threads with new episodes, recover
     it.
3. **Size by shared memory and registers first, device memory last.**
   On these cards the 8 GB is for running many experiments at once,
   not one huge one.
4. **Measure the register limit,** not the occupancy: the best here
   was the tightest limit that does not spill.
5. **Use counter-based randomness.** It is free to parallelise and
   replays exactly.
6. **Keep the past on the card.** A hall of fame turns self-play from
   a chase into a curriculum. Alone, it was not enough here. The
   learner also needed opponents better than its own early selves (the
   sparring bots) to have something to climb against.

### E. Threats to validity

- **One seed per arm, and the arms compared across cards.** The
  replicate on calber fell short of two standard errors. The effect is
  likely real but smaller than whitewolf's run shows.
- **The yardstick is held out in seeds, not in kind.** Its skill lies
  inside the sparring range. A different scripted style, or treadcore's
  own brain, would be a stricter judge.
- **Not equally long.** The no-sparring run is 300 generations, the
  others 400. At equal age (260 to 300) the verdicts are the same.
- **"net GFLOP/s" counts only the net's arithmetic** (VI.C).
- **boburner's clock and power were not logged** during its benchmark.

---

## VIII. Conclusions

- **It can be done.** A self-learning loop with sensors, a population,
  selection, mutation and a memory of 4,096 past champions runs entirely
  on each of the fleet's cards.
- **Speed.** 419 to 581 M tank-steps a second per card; 1.46 G for the
  fleet.
- **The decisive setting.** The register limit doubled the speed.
- **The limits.** Shared memory limits the net's width; memory does
  not limit the population.
- **Learning.** It is slow but measurable: the champion went from
  winning 0 % to winning 22 % against a scripted tank in 400
  generations on one card, against 8 % without sparring partners.

**Next:**
- **The interface redesign** (the rest of the prompt). It will treat
  crucible as orrery's second project.
- **Islands across the three cards,** as a run kind in the queue.
- **The headroom in VII.B,** measured one change at a time against
  this report's numbers.

The runs' files are in `runs/` (TSV per generation, `nvidia-smi` logs,
and the console). The states (54 MB each) stay on the nodes.
