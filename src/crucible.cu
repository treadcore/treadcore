// crucible: a self-learning loop that never leaves the GPU.
//
// Paul, 28 October: "Is there a neural net that we can train and retrain to
// keep its generations within itself. We need sensor data and a self
// contained loop ... specifically targeted for CUDA program that will stress
// the cards ... How much of this simulation can be ran fully in cuda itself."
//
// All of it. The host sets up memory, launches kernels and reads a few
// numbers back each generation; everything else is on the card:
//
//   the world      a 64 x 64 arena, six pillars, two tanks, one shell each
//   the sensors    twelve rays round each tank (wall or pillar, and the enemy),
//                  bearing to the enemy, its own speed, hit points, reload
//   the brain      a small net (32 in, two hidden layers of H, 3 out: turn,
//                  throttle, fire), one per individual
//   the loop       a generation: every individual plays EPS duels at once
//                  against an opponent drawn from the hall of fame; the
//                  population is sorted by score; the best E are kept, the
//                  rest are mutated copies of the top quarter; every K
//                  generations the champion joins the hall of fame
//   the memory     the hall of fame is a ring of CAP champions on the card:
//                  its generations stay within itself, and the learner keeps
//                  playing its own past, so it does not forget how to beat it
//
// The mapping is one block per individual and one thread per duel. The
// block's net and its opponent's sit in shared memory, so every thread of
// a warp reads the same weight at the same time (a broadcast): the work is
// arithmetic, not memory traffic, which is what stresses a card.
//
// A fixed yardstick, a scripted tank that turns to the enemy, closes and
// fires when aimed, is played by the champion every --yard generations:
// its win rate against the yardstick is the evidence of learning, since
// scores against the hall of fame rise and fall with the hall itself.
//
// The ladder (Paul, 28 October: "islands' own past champions ... battling
// known milestones in the past is a good idea"): once the islands beat the
// yardstick nearly every time it measures nothing, so the champions the
// fleet has kept as milestones are the harder yardstick. --ladder loads
// them, oldest first. In training they are opponents too (--lad, the
// chance of one), so a learner keeps beating the fleet's whole past, not
// only its own hall's last few thousand. Every --yard generations the
// champion's score against the ladder is written beside the yardstick's,
// and --exam writes the final champion's score against each milestone,
// the evidence the coordinator uses to make it the next one.
//
// THE WORLD (make WORLD=1; Paul, 3 November: "the rules of this world, can
// we apply them to the simulation expand the matrix dimensions. to the
// crucible"). The arena above is a toy with its own numbers: speed 6, a gun
// that reaches 35, three hits of 1, no ammunition, eyes that see the whole
// arena. Under WORLD it plays treadcore's numbers instead (treadcore-sim's
// rules.rs, one arena unit a tile of 25 m): speed 4 tiles/s reached at 4
// and shed at 6 a second, the hull turning 3 rad/s and easing off for a
// corner as treadcore's driver does, 100 of armour, 24 shells, a gun of 10
// tiles at 40 tiles/s for 45 a hit and a 4 s reload, own eyes of 10 tiles,
// and four NEUTRAL PILLBOXES in the middle band that shoot the nearest tank
// they can see within 8 for 15 every 2 s and take two shells to kill. The
// pillbox is the rule that matters: treadcore's learned nets die in the
// first minute to a neutral pillbox's ring, and this arena never had one.
// What is left out is said, not hidden: forest, roads, water, the indirect
// gun, drones, ore and the economy are not here; a duel has no map.
//
// The net grows to see the world: 20 senses more (a ray each for pillboxes,
// the nearest one's bearing, nearness, readiness, health and whether it has
// you, and the tank's own shells and reload), 52 inputs, 55 with HYBRID's
// three rules. EXPANDING THE MATRIX KEEPS WHAT WAS LEARNED. A state or a
// champion of a smaller net (fewer inputs, a narrower H, fewer outputs) is
// loaded into this one with every new weight zero: the new inputs reach
// nothing, the new hidden units add tanh(0) = 0, and the old net's answers
// come out unchanged, so 160,000 generations of an island are where the
// world island starts, not a random net. Mutation then wakes the new
// weights. A larger net never shrinks into a smaller one: that would lose
// weights, and it is refused with a sentence.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cmath>
#include <string>
#include <vector>
#include <chrono>
#include <algorithm>
#include <map>
#include <cctype>
#include <dirent.h>
#include <map>
#include <cctype>
#include <dirent.h>
#include <cuda_runtime.h>
#include <thrust/device_ptr.h>
#include <thrust/sequence.h>
#include <thrust/sort.h>
#include <thrust/reduce.h>
#include <thrust/functional.h>

#ifndef HID
#define HID 32
#endif
#define H HID
// Blocks of 256 that each SM must hold at once: the compiler limits each
// thread's registers to fit (1: no limit; 2: 128; 4: 64, spilling).
#ifndef MINB
#define MINB 1
#endif
// HYBRID (make HYBRID=1): the same design as treadcore's Algorhythm. The scripted tank
// decides first; the net reads the senses and that answer (3 more inputs) and gives a
// correction and a confidence (1 more output). The gate is shut below zero, and then the
// tank plays the rules exactly. A state file of one kind does not load in the other: NW differs.
#ifdef HYBRID
#define RULES_IN 3
#define NOUT 4
#else
#define RULES_IN 0
#define NOUT 3
#endif
// WORLD's senses come after HYBRID's rules, so the first 32 (and a hybrid's
// 35) keep their places and a smaller net's weights land where they were.
#ifdef WORLD
#define WORLD_IN 20
#else
#define WORLD_IN 0
#endif
#if defined(FOREST) && !defined(WORLD)
#error "FOREST is a world arena's: build with WORLD=1 FOREST=1"
#endif
#define AT_WORLD (32 + RULES_IN)
#define NIN (32 + RULES_IN + WORLD_IN)
#define NW (NIN * H + H + H * H + H + H * NOUT + NOUT)
// Two nets sit in a block's shared memory, and a Pascal card gives a block
// 48 KB: H=48 with WORLD and HYBRID (55 in, 4 out) is 41,888 bytes; H=64
// would be 64,032, and the launch would fail at run time, so it fails here.
static_assert(2 * NW * sizeof(float) <= 49152, "two nets must fit 48 KB of shared memory: build with a smaller H");
#define RAYS 12
#define PILLARS 6
#define ARENA 64.0f
#define RANGE 40.0f
#define DT 0.1f
#ifdef WORLD
// treadcore-sim's rules.rs, by name. EACH MAY BE GIVEN AT BUILD TIME
// (make R_VMAX=4.5f ...): the coordinator reads rules.rs at the
// fleet's commit and passes every one, so the world the islands train in is
// the game's as it is now, not as it was when this file was written (Paul,
// 4 November: "the GPU build always looks like the perfect world in
// treadcore"). The numbers here are the game's of 3 November, for a build
// by hand.
#ifndef VMAX
#define VMAX 4.0f              // TANK_SPEED, tiles/s
#endif
#ifndef ACCEL
#define ACCEL 4.0f             // TANK_ACCEL
#endif
#ifndef BRAKE
#define BRAKE 6.0f             // TANK_BRAKE
#endif
#ifndef TURN
#define TURN 3.0f              // TANK_TURN, rad/s
#endif
#define TURN_BRAKE_ANGLE 0.6f  // a heading error past this eases off
#define TURN_CRAWL 0.35f       // down to this share of speed for a turn right round
#define TURN_PIVOT 2.0f        // past this it pivots where it stands
#ifndef GUN_RANGE
#define GUN_RANGE 10.0f        // DIRECT_RANGE
#endif
#ifndef SSPEED
#define SSPEED 40.0f           // SHELL_SPEED_DIRECT
#endif
#define SLIFE (GUN_RANGE / SSPEED)
#ifndef RELOAD
#define RELOAD 4.0f            // RELOAD_TICKS, in seconds
#endif
#ifndef HP
#define HP 100.0f              // TANK_ARMOR
#endif
#ifndef DAMAGE
#define DAMAGE 45.0f           // DIRECT_DAMAGE: a hit, the unit fitness counts in
#endif
#ifndef HIT_R
#define HIT_R 0.8f             // DIRECT_HIT_RADIUS
#endif
#ifndef SHELLS
#define SHELLS 24.0f           // TANK_SHELLS
#endif
#ifndef VISION
#define VISION 10.0f           // TANK_VISION: own eyes, as far as the gun
#endif
#define PILLBOXES 4
#ifndef PB_RANGE
#define PB_RANGE 8.0f          // PILLBOX_RANGE
#endif
#ifndef PB_DAMAGE
#define PB_DAMAGE 15.0f        // PILLBOX_DAMAGE
#endif
#ifndef PB_RELOAD
#define PB_RELOAD 2.0f         // PILLBOX_RELOAD_TICKS, in seconds
#endif
#ifndef PB_HEALTH
#define PB_HEALTH 60.0f        // PILLBOX_HEALTH: two direct hits
#endif
#define PB_HIT (HIT_R + 0.5f)  // a shell strikes a pillbox this near, as treadcore's does
#define PB_R 0.5f              // its body, to drive round
#define STEPS_DEFAULT 1200     // two minutes: with a 4 s reload, 600 steps is fifteen shots
// THE TREES (Paul, 6 November: "The CUDA needs to have trees and more training aspects"). FOREST=1
// lays GROVES groves of trees in the arena, each a disc, and they do to a duel what treadcore's forest
// does to a game (treadcore-sim, map.rs and lib.rs): a tank in one goes at 1/FOREST_COST of its speed
// (Terrain::Forest's move_cost, 1.8); a flat shot does not pass through one (blocks_fire: the shell
// stops in the trees, as line_clear refuses a shot across them); a tank deep in one -- treadcore's 2 x 2
// block of forest, CONCEAL_DEPTH here -- is hidden from every eye and every pillbox beyond CONCEAL_RANGE.
// GROVE_EDGE is the half tile of a grove's edge a shot may skim: line_clear samples a line every half
// tile from half a tile out, so a tank on a forest's edge can still shoot and be shot.
// THE NET'S SENSES DO NOT CHANGE, and that is deliberate: the game's Gladiator (treadcore-brain,
// styles.rs world_senses) casts its rays to impassable terrain only, so forest is invisible to its rays
// as it is here; a tank in the trees is simply not seen, and a shot into them is simply lost. A champion
// of a forest arena is the same 55 numbers in the game, and the bridge can still read it.
#ifdef FOREST
#define GROVES 6
#ifndef CONCEAL_RANGE
#define CONCEAL_RANGE 3.0f     // CONCEAL_RANGE
#endif
#define FOREST_COST 1.8f       // Terrain::Forest's move_cost (map.rs, not rules.rs: crucible's own here)
#define GROVE_EDGE 0.5f
#define CONCEAL_DEPTH 1.0f
// WALLS AND THE TAKEOVER (Paul, 6 November: "navigate a simple maze ... S shaped and 5 shaped, and @ ...
// where we add takeover tasks like Pillbox capture, Base capture, Building a base capture"). A wall is a
// box the rays see (the Gladiator's rays stop at impassable terrain, walls among it), a tank cannot cross
// and a shot cannot pass; a training arena lays TRAIN_WALLS bars, a gauntlet's maze up to WALLS. A pillbox
// shot to rubble is TAKEN by the tank that drives onto it (within CAPTURE_R): it stands again, its
// taker's, and fires only at the other tank, as a squad's taken pillbox does in treadcore.
#define WALLS 12
#define TRAIN_WALLS 3
#define CAPTURE_R 1.8f
// THE GAME'S SCORE, IN KILLS (treadcore-sim rules.rs: SCORE_KILL 100, SCORE_CAPTURE 25, SCORE_HOLD 2 for
// each live structure every HOLD_EVERY_TICKS, 10 s): a kill 1, a capture 0.25, a held pillbox 0.02 every
// 10 s. A forest arena's fitness adds it, so a champion is chosen by what the game would score it, beside
// the duel's own terms; each may be given at build time as the rules are.
#ifndef SCORE_KILL_W
#define SCORE_KILL_W 1.0f
#endif
#ifndef SCORE_CAPTURE_W
#define SCORE_CAPTURE_W 0.25f
#endif
#ifndef SCORE_HOLD_W
#define SCORE_HOLD_W 0.02f
#endif
#define HOLD_EVERY_S 10.0f
#endif
#else
#define VMAX 6.0f
#define TURN 2.5f
#define SSPEED 30.0f
#define SLIFE 1.2f
#define RELOAD 1.0f
#define HP 3.0f
#define STEPS_DEFAULT 600
#endif
#define TANK_R 1.0f

#define CK(x) do { cudaError_t e_ = (x); if (e_ != cudaSuccess) { fprintf(stderr, "%s:%d %s\n", __FILE__, __LINE__, cudaGetErrorString(e_)); exit(1); } } while (0)

// ---- randomness: counter-based, so any thread can draw any number -----------

__host__ __device__ inline uint32_t hash32(uint32_t x) {
    x ^= x >> 16; x *= 0x7feb352du; x ^= x >> 15; x *= 0x846ca68bu; x ^= x >> 16;
    return x;
}
__host__ __device__ inline uint32_t mix(uint32_t a, uint32_t b) { return hash32(a ^ (hash32(b) + 0x9e3779b9u + (a << 6) + (a >> 2))); }

struct Rng {
    uint32_t s;
    __device__ float u() { s = hash32(s + 0x9e3779b9u); return (s >> 8) * (1.0f / 16777216.0f); }
    __device__ float in(float a, float b) { return a + (b - a) * u(); }
};

__device__ inline float gauss(uint32_t key) {
    uint32_t h1 = hash32(key), h2 = hash32(key ^ 0x68bc21ebu);
    float u1 = ((h1 >> 8) + 1) * (1.0f / 16777217.0f);
    float u2 = (h2 >> 8) * (1.0f / 16777216.0f);
    return sqrtf(-2.0f * __logf(u1)) * __cosf(6.2831853f * u2);
}

// ---- the world ------------------------------------------------------------------

struct Tank {
    float x, y, a, v, hp, reload;
    float sx, sy, sdx, sdy, slife; // its shell
#ifdef WORLD
    float shells;
#endif
#ifdef FOREST
    int side;                      // 0 the learner, 1 the other: whose a taken pillbox is
#endif
};

struct Arena {
    float px[PILLARS], py[PILLARS], pr[PILLARS];
#ifdef WORLD
    // The neutral pillboxes: where, health, and seconds until the next shot.
    float bx[PILLBOXES], by[PILLBOXES], bhp[PILLBOXES], brl[PILLBOXES];
#endif
#ifdef FOREST
    // The groves: centre and radius, tiles.
    float gx[GROVES], gy[GROVES], gr[GROVES];
    // The walls, boxes from (x0, y0) to (x1, y1), the first nwall of them; and each pillbox's owner: -1
    // neutral, 0 the learner's, 1 the other tank's (taken from rubble).
    float wx0[WALLS], wy0[WALLS], wx1[WALLS], wy1[WALLS];
    int nwall;
    int bown[PILLBOXES];
#endif
};

__device__ inline float ray_circle(float ox, float oy, float dx, float dy, float cx, float cy, float r) {
    float fx = ox - cx, fy = oy - cy;
    float b = fx * dx + fy * dy;
    float c = fx * fx + fy * fy - r * r;
    float disc = b * b - c;
    if (disc < 0.0f) return 1e9f;
    float t = -b - sqrtf(disc);
    return t > 0.0f ? t : 1e9f;
}

__device__ inline float ray_wall(float ox, float oy, float dx, float dy) {
    float t = 1e9f;
    if (dx > 1e-6f) t = fminf(t, (ARENA - ox) / dx); else if (dx < -1e-6f) t = fminf(t, -ox / dx);
    if (dy > 1e-6f) t = fminf(t, (ARENA - oy) / dy); else if (dy < -1e-6f) t = fminf(t, -oy / dy);
    return t;
}

// Where a ray first meets a box (0 when it starts inside; 1e9 when it never does): the slab test.
__device__ inline float ray_box(float ox, float oy, float dx, float dy, float x0, float y0, float x1, float y1) {
    float lo = -1e9f, hi = 1e9f;
    if (fabsf(dx) < 1e-6f) { if (ox < x0 || ox > x1) return 1e9f; }
    else { float a = (x0 - ox) / dx, b = (x1 - ox) / dx; lo = fmaxf(lo, fminf(a, b)); hi = fminf(hi, fmaxf(a, b)); }
    if (fabsf(dy) < 1e-6f) { if (oy < y0 || oy > y1) return 1e9f; }
    else { float a = (y0 - oy) / dy, b = (y1 - oy) / dy; lo = fmaxf(lo, fminf(a, b)); hi = fminf(hi, fmaxf(a, b)); }
    if (hi < fmaxf(lo, 0.0f)) return 1e9f;
    return lo > 0.0f ? lo : 0.0f;
}

#ifdef WORLD
// A circle along a ray, entered from outside or already around the start
// (0): a shell fired point-blank strikes, it does not pass through.
__device__ inline float ray_circle_in(float ox, float oy, float dx, float dy, float cx, float cy, float r) {
    float fx = ox - cx, fy = oy - cy;
    float c = fx * fx + fy * fy - r * r;
    if (c <= 0.0f) return 0.0f;
    float b = fx * dx + fy * dy;
    float disc = b * b - c;
    if (disc < 0.0f) return 1e9f;
    float t = -b - sqrtf(disc);
    return t > 0.0f ? t : 1e9f;
}

// treadcore's line_clear, for a duel without a map: no pillar between.
__device__ inline bool line_clear(float ax, float ay, float bx, float by, const Arena& w) {
    float dx = bx - ax, dy = by - ay;
    float d = sqrtf(dx * dx + dy * dy);
    if (d < 1e-4f) return true;
    dx /= d; dy /= d;
#pragma unroll
    for (int p = 0; p < PILLARS; p++)
        if (ray_circle(ax, ay, dx, dy, w.px[p], w.py[p], w.pr[p]) < d) return false;
#ifdef FOREST
    for (int b = 0; b < w.nwall; b++)
        if (ray_box(ax, ay, dx, dy, w.wx0[b], w.wy0[b], w.wx1[b], w.wy1[b]) < d) return false;
    // Forest between, past the half tile at either end: the chord of the line through a grove's core.
#pragma unroll
    for (int g = 0; g < GROVES; g++) {
        float r = w.gr[g] - GROVE_EDGE;
        if (r <= 0.0f) continue;
        float fx = ax - w.gx[g], fy = ay - w.gy[g];
        float b = fx * dx + fy * dy, c = fx * fx + fy * fy - r * r, disc = b * b - c;
        if (disc <= 0.0f) continue;
        float q = sqrtf(disc), t0 = -b - q, t1 = -b + q;
        if (t1 > GROVE_EDGE && t0 < d - GROVE_EDGE) return false;
    }
#endif
    return true;
}

// In the trees: a point inside a grove by at least `depth` tiles.
__device__ inline bool in_forest(float x, float y, const Arena& w, float depth) {
#ifdef FOREST
#pragma unroll
    for (int g = 0; g < GROVES; g++) {
        float r = w.gr[g] - depth, dx = x - w.gx[g], dy = y - w.gy[g];
        if (r > 0.0f && dx * dx + dy * dy < r * r) return true;
    }
#endif
    return false;
}

// A pillbox this tank has taken: its own, not a threat (the Gladiator's senses leave out allied ones).
__device__ inline bool mine(const Arena& w, int k, const Tank& me) {
#ifdef FOREST
    return w.bown[k] == me.side;
#else
    return false;
#endif
}

// treadcore's concealment (tank_seen_by, the pillboxes' choice of target): a tank deep in the trees is
// seen only from inside CONCEAL_RANGE. Without FOREST nothing hides.
__device__ inline bool hidden_from(float x, float y, float dist, const Arena& w) {
#ifdef FOREST
    return dist > CONCEAL_RANGE && in_forest(x, y, w, CONCEAL_DEPTH);
#else
    return false;
#endif
}
#endif

__device__ inline float wrap(float a) {
    while (a > 3.14159265f) a -= 6.2831853f;
    while (a < -3.14159265f) a += 6.2831853f;
    return a;
}

// The sensors: what tank `me` can know, as 32 numbers in 0..1 or -1..1
// (and under WORLD twenty more, from AT_WORLD).
__device__ void sense(const Tank& me, const Tank& en, const Arena& w, float* in) {
#pragma unroll
    for (int r = 0; r < RAYS; r++) {
        float ang = me.a + r * (6.2831853f / RAYS);
        float dx = __cosf(ang), dy = __sinf(ang);
        float tw = ray_wall(me.x, me.y, dx, dy);
#pragma unroll
        for (int p = 0; p < PILLARS; p++) tw = fminf(tw, ray_circle(me.x, me.y, dx, dy, w.px[p], w.py[p], w.pr[p]));
#ifdef FOREST
        for (int b = 0; b < w.nwall; b++) tw = fminf(tw, ray_box(me.x, me.y, dx, dy, w.wx0[b], w.wy0[b], w.wx1[b], w.wy1[b]));
#endif
        float te = en.hp > 0.0f ? ray_circle(me.x, me.y, dx, dy, en.x, en.y, TANK_R) : 1e9f;
#ifdef WORLD
        // Own eyes reach VISION: the walls and pillars are the map, known;
        // a tank further off is not seen, in either channel.
        if (te > VISION) te = 1e9f;
        if (te < 1e8f && hidden_from(en.x, en.y, te, w)) te = 1e9f;
        float tp = 1e9f;
#pragma unroll
        for (int k = 0; k < PILLBOXES; k++)
            if (w.bhp[k] > 0.0f && !mine(w, k, me)) tp = fminf(tp, ray_circle(me.x, me.y, dx, dy, w.bx[k], w.by[k], PB_HIT));
        in[AT_WORLD + r] = (tp < tw && tp < RANGE) ? 1.0f - tp / RANGE : 0.0f;
#endif
        float near = fminf(fminf(tw, te), RANGE);
        in[2 * r] = 1.0f - near / RANGE;
        in[2 * r + 1] = (te < tw && te < RANGE) ? 1.0f - te / RANGE : 0.0f;
    }
    float b = wrap(atan2f(en.y - me.y, en.x - me.x) - me.a);
    float ex = en.sx - me.x, ey = en.sy - me.y;
    float sd = sqrtf(ex * ex + ey * ey);
    in[24] = me.v / VMAX;
    in[25] = me.hp / HP;
    in[26] = me.reload <= 0.0f ? 1.0f : 0.0f;
    in[27] = en.hp / HP;
    in[28] = __sinf(b);
    in[29] = __cosf(b);
    in[30] = (en.slife > 0.0f && sd < 10.0f) ? 1.0f - sd / 10.0f : 0.0f;
    in[31] = 1.0f;
#ifdef WORLD
    // The nearest live pillbox, and the tank's own gun. Twenty in all with
    // the twelve rays above: AT_WORLD + 0..11 rays, 12..17 the pillbox,
    // 18..19 the gun.
    float* x = in + AT_WORLD;
    int nk = -1;
    float nd = 1e9f;
#pragma unroll
    for (int k = 0; k < PILLBOXES; k++) {
        if (w.bhp[k] <= 0.0f || mine(w, k, me)) continue;
        float dx = w.bx[k] - me.x, dy = w.by[k] - me.y;
        float d = sqrtf(dx * dx + dy * dy);
        if (d < nd) { nd = d; nk = k; }
    }
    if (nk >= 0) {
        float pb = wrap(atan2f(w.by[nk] - me.y, w.bx[nk] - me.x) - me.a);
        x[12] = __sinf(pb);
        x[13] = __cosf(pb);
        x[14] = fmaxf(0.0f, 1.0f - nd / RANGE);
        x[15] = fmaxf(w.brl[nk], 0.0f) / PB_RELOAD; // 0: it can fire now
        x[16] = w.bhp[nk] / PB_HEALTH;
        x[17] = (nd <= PB_RANGE && line_clear(w.bx[nk], w.by[nk], me.x, me.y, w)) ? 1.0f : 0.0f; // it has you
    } else {
        x[12] = x[13] = x[14] = x[15] = x[16] = x[17] = 0.0f;
    }
    x[18] = me.shells / SHELLS;
    x[19] = fmaxf(me.reload, 0.0f) / RELOAD;
#endif
}

// The brain: NIN -> H -> H -> NOUT, tanh throughout. `w` is in shared memory and
// every thread of the warp reads the same element at once.
__device__ void think(const float* __restrict__ w, const float* in, float* out) {
    const float* W1 = w;
    const float* b1 = W1 + NIN * H;
    const float* W2 = b1 + H;
    const float* b2 = W2 + H * H;
    const float* W3 = b2 + H;
    const float* b3 = W3 + H * NOUT;
    float h1[H], h2[H];
#pragma unroll
    for (int j = 0; j < H; j++) {
        float s = b1[j];
#pragma unroll
        for (int i = 0; i < NIN; i++) s = fmaf(W1[j * NIN + i], in[i], s);
        h1[j] = tanhf(s);
    }
#pragma unroll
    for (int j = 0; j < H; j++) {
        float s = b2[j];
#pragma unroll
        for (int i = 0; i < H; i++) s = fmaf(W2[j * H + i], h1[i], s);
        h2[j] = tanhf(s);
    }
#pragma unroll
    for (int k = 0; k < NOUT; k++) {
        float s = b3[k];
#pragma unroll
        for (int i = 0; i < H; i++) s = fmaf(W3[k * H + i], h2[i], s);
        out[k] = tanhf(s);
    }
}

// A scripted tank: turn to the enemy, close to its range, fire when aimed
// within `tol` radians. The yardstick is (0.12, 11); a sparring partner
// draws both per block, so the learner meets bots good and bad.
// A scripted tank's skill: how true it must aim before it fires, the range it stands off at, and
// under WORLD its style (STYLES: 0 the script as it always was, the yardstick's and the hybrid's own).
struct Skill { float tol, range; int style; };
#ifdef WORLD
// Inside the gun's 10, as a treadcore bot stands off; sparring partners
// stand anywhere from 4 to 9.
#define YARDSTICK Skill{0.12f, 8.0f, 0}
#ifdef FOREST
#define STYLES 6               // and 5, the Lurker: waits in the trees' edge and shoots from cover
#else
#define STYLES 5               // the sparring partners' styles: the script, Raider, Castle, Expander, Timid
#endif
#define SPAR_NEAR 4.0f
#define SPAR_FAR 9.0f
#define FIRE_REACH GUN_RANGE
#else
#define YARDSTICK Skill{0.12f, 11.0f, 0}
#define SPAR_NEAR 6.0f
#define SPAR_FAR 20.0f
#define FIRE_REACH 35.0f
#endif

__device__ void scripted(const Tank& me, const Tank& en, Skill k, const Arena& w, float* out) {
    float dx = en.x - me.x, dy = en.y - me.y;
    float d = sqrtf(dx * dx + dy * dy);
    float b = wrap(atan2f(dy, dx) - me.a);
#ifdef WORLD
    // Under WORLD the turn is a heading error (see drive): the bearing itself.
    out[0] = fmaxf(-1.0f, fminf(1.0f, b / 3.14159265f));
#else
    out[0] = fmaxf(-1.0f, fminf(1.0f, 2.0f * b));
#endif
    out[1] = d > k.range + 3.0f ? 1.0f : (d < k.range - 3.0f ? -0.5f : 0.0f);
    out[2] = (fabsf(b) < k.tol && d < FIRE_REACH) ? 1.0f : -1.0f;
#ifdef WORLD
    // THE GAME'S STYLES (Paul, 4 November, of the Gladiator's gap: "Islands meet the game's styles").
    // A sparring partner plays one of treadcore's rules styles as a duel can show it, so a champion
    // learns the fights the game will give it: 1 Raider, goes and fights, closes to four and presses
    // while it is not the weaker or the enemy is hurt; 2 Castle, holds its ground and fires on what
    // comes into reach, never chasing; 3 Expander, takes the map, the nearest live pillbox shelled
    // first; 4 Timid, keeps away unless the odds are well on its side. 0 is the script above.
    if (k.style == 1) {
        if (me.hp >= en.hp || en.hp < 0.5f * HP) out[1] = d > 4.0f ? 1.0f : 0.0f;
    } else if (k.style == 2) {
        out[1] = 0.0f;
    } else if (k.style == 3) {
        int nk = -1;
        float nd = 1e9f;
        for (int q = 0; q < PILLBOXES; q++) {
            if (w.bhp[q] <= 0.0f) continue;
            float px = w.bx[q] - me.x, py = w.by[q] - me.y, pd = sqrtf(px * px + py * py);
            if (pd < nd) { nd = pd; nk = q; }
        }
        if (nk >= 0) {
            float pb = wrap(atan2f(w.by[nk] - me.y, w.bx[nk] - me.x) - me.a);
            out[0] = fmaxf(-1.0f, fminf(1.0f, pb / 3.14159265f));
            out[1] = nd > GUN_RANGE - 2.0f ? 1.0f : 0.0f;
            out[2] = (fabsf(pb) < k.tol && nd < GUN_RANGE) ? 1.0f : -1.0f;
        }
    } else if (k.style == 4) {
        if (!(me.hp > en.hp + 0.3f * HP) && d < 14.0f) {
            float away = wrap(b + 3.14159265f);
            out[0] = fmaxf(-1.0f, fminf(1.0f, away / 3.14159265f));
            out[1] = 1.0f;
        }
    }
#endif
#ifdef FOREST
    // 5, THE LURKER: into the nearest grove on the side that faces the enemy, deep enough to be hidden
    // (CONCEAL_DEPTH and a little), and when the enemy comes within the gun's reach out to the edge, where
    // its shot leaves the trees, to fire. A champion that meets one learns that an empty clearing may not
    // be empty. With no grove it is a Castle.
    if (k.style == 5) {
        int ng = -1;
        float gd = 1e9f;
        for (int g = 0; g < GROVES; g++) {
            float gx = w.gx[g] - me.x, gy = w.gy[g] - me.y, dd = sqrtf(gx * gx + gy * gy) - w.gr[g];
            if (dd < gd) { gd = dd; ng = g; }
        }
        out[1] = 0.0f;
        if (ng >= 0) {
            float ex = en.x - w.gx[ng], ey = en.y - w.gy[ng], el = fmaxf(sqrtf(ex * ex + ey * ey), 1e-3f);
            // Hidden while the enemy is out of reach; when it comes within the gun's reach, out to the
            // edge it faces, where a flat shot leaves the trees (treadcore refuses one from inside them).
            float depth = d < GUN_RANGE + 1.0f ? 0.2f : fminf(CONCEAL_DEPTH + 0.3f, w.gr[ng] * 0.5f);
            float tx = w.gx[ng] + ex / el * (w.gr[ng] - depth), ty = w.gy[ng] + ey / el * (w.gr[ng] - depth);
            float sx = tx - me.x, sy = ty - me.y, sd = sqrtf(sx * sx + sy * sy);
            if (sd > 0.6f) {
                float sb = wrap(atan2f(sy, sx) - me.a);
                out[0] = fmaxf(-1.0f, fminf(1.0f, sb / 3.14159265f));
                out[1] = sd > 2.0f ? 1.0f : 0.4f;
            }
        }
    }
    // WHAT THE TREES DO TO EVERY STYLE: an enemy hidden in them is not seen, so not steered at or shot
    // at (the tank keeps its heading); and treadcore refuses a flat shot with forest in the way, so a
    // scripted tank does not fire through trees. The Expander's shot at a pillbox needs the same line.
    if (k.style != 3) {
        if (hidden_from(en.x, en.y, d, w)) { out[0] = 0.0f; out[2] = -1.0f; }
        else if (out[2] > 0.0f && !line_clear(me.x, me.y, en.x, en.y, w)) out[2] = -1.0f;
    } else if (out[2] > 0.0f) {
        int nk = -1;
        float nd = 1e9f;
        for (int q = 0; q < PILLBOXES; q++) {
            if (w.bhp[q] <= 0.0f) continue;
            float px = w.bx[q] - me.x, py = w.by[q] - me.y, pd = sqrtf(px * px + py * py);
            if (pd < nd) { nd = pd; nk = q; }
        }
        if (nk >= 0 && !line_clear(me.x, me.y, w.bx[nk], w.by[nk], w)) out[2] = -1.0f;
    }
#endif
}

// What a net-driven tank does. Returns 1 when the net's own say reached the tank.
__device__ int decide(const float* __restrict__ wt, const Tank& me, const Tank& en, const Arena& w, float* in, float* out) {
    sense(me, en, w, in);
#ifdef HYBRID
    float rules[3];
    scripted(me, en, YARDSTICK, w, rules);
    in[32] = rules[0]; in[33] = rules[1]; in[34] = rules[2];
    think(wt, in, out);
    bool open = out[3] > 0.0f;
#pragma unroll
    for (int i = 0; i < 3; i++) out[i] = open ? fmaxf(-1.0f, fminf(1.0f, rules[i] + out[i])) : rules[i];
    return open ? 1 : 0;
#else
    think(wt, in, out);
    return 1;
#endif
}

__device__ void drive(Tank& t, const float* act, const Arena& w) {
    if (t.hp <= 0.0f) return;
#ifdef WORLD
    // treadcore's driver (treadcore-sim, the tank's step): the turn output
    // names a heading error, as a waypoint does there (-1..1 is -pi..pi);
    // the hull turns toward it at most TURN a second; past TURN_BRAKE_ANGLE
    // the tank eases off, to TURN_CRAWL for a turn right round, and past
    // TURN_PIVOT it pivots where it stands. Speed comes up at ACCEL and goes
    // down at BRAKE. There is no reverse in treadcore, so none here.
    float err = act[0] * 3.14159265f, ae = fabsf(err);
    float top = VMAX;
    if (ae > TURN_BRAKE_ANGLE) {
        float over = fminf(fmaxf((ae - TURN_BRAKE_ANGLE) / (3.14159265f - TURN_BRAKE_ANGLE), 0.0f), 1.0f);
        top = ae > TURN_PIVOT ? 0.0f : VMAX * (1.0f - over * (1.0f - TURN_CRAWL));
    }
#ifdef FOREST
    // Terrain::Forest's move_cost: a tank in the trees goes at 1/1.8 of its speed.
    if (in_forest(t.x, t.y, w, 0.0f)) top /= FOREST_COST;
#endif
    float want = fminf(fmaxf(act[1], 0.0f) * VMAX, top);
    t.v = t.v < want ? fminf(t.v + ACCEL * DT, want) : fmaxf(t.v - BRAKE * DT, want);
    t.a = wrap(t.a + fminf(fmaxf(err, -TURN * DT), TURN * DT));
#else
    t.a = wrap(t.a + act[0] * TURN * DT);
    t.v += (act[1] * VMAX - t.v) * 0.3f;
#endif
    t.x = fminf(fmaxf(t.x + __cosf(t.a) * t.v * DT, TANK_R), ARENA - TANK_R);
    t.y = fminf(fmaxf(t.y + __sinf(t.a) * t.v * DT, TANK_R), ARENA - TANK_R);
#pragma unroll
    for (int p = 0; p < PILLARS; p++) {
        float dx = t.x - w.px[p], dy = t.y - w.py[p];
        float d = sqrtf(dx * dx + dy * dy), m = w.pr[p] + TANK_R;
        if (d < m && d > 1e-4f) { t.x = w.px[p] + dx / d * m; t.y = w.py[p] + dy / d * m; t.v *= 0.5f; }
    }
#ifdef FOREST
    for (int b = 0; b < w.nwall; b++) {
        float cx = fminf(fmaxf(t.x, w.wx0[b]), w.wx1[b]), cy = fminf(fmaxf(t.y, w.wy0[b]), w.wy1[b]);
        float dx = t.x - cx, dy = t.y - cy, d = sqrtf(dx * dx + dy * dy);
        if (d < TANK_R) {
            if (d > 1e-4f) { t.x = cx + dx / d * TANK_R; t.y = cy + dy / d * TANK_R; }
            else { t.x -= __cosf(t.a) * t.v * DT; t.y -= __sinf(t.a) * t.v * DT; }
            t.v *= 0.5f;
        }
    }
#endif
#ifdef WORLD
    // A pillbox, live or rubble, is driven round as a pillar is.
#pragma unroll
    for (int k = 0; k < PILLBOXES; k++) {
        float dx = t.x - w.bx[k], dy = t.y - w.by[k];
        float d = sqrtf(dx * dx + dy * dy), m = PB_R + TANK_R;
        if (d < m && d > 1e-4f) { t.x = w.bx[k] + dx / d * m; t.y = w.by[k] + dy / d * m; t.v *= 0.5f; }
    }
#endif
    t.reload -= DT;
#ifdef WORLD
    if (act[2] > 0.0f && t.reload <= 0.0f && t.slife <= 0.0f && t.shells >= 1.0f) {
        t.shells -= 1.0f;
#else
    if (act[2] > 0.0f && t.reload <= 0.0f && t.slife <= 0.0f) {
#endif
        t.sdx = __cosf(t.a) * SSPEED; t.sdy = __sinf(t.a) * SSPEED;
        t.sx = t.x + __cosf(t.a) * (TANK_R + 0.2f); t.sy = t.y + __sinf(t.a) * (TANK_R + 0.2f);
        t.slife = SLIFE; t.reload = RELOAD;
    }
}

// A shell's flight, a tenth of a second in two halves so it cannot pass
// through a tank. Returns 1 for a hit.
__device__ int fly(Tank& t, Tank& en, const Arena& w) {
    if (t.slife <= 0.0f) return 0;
#pragma unroll
    for (int k = 0; k < 2; k++) {
        t.sx += t.sdx * DT * 0.5f; t.sy += t.sdy * DT * 0.5f;
        if (t.sx < 0 || t.sy < 0 || t.sx > ARENA || t.sy > ARENA) { t.slife = 0; return 0; }
#pragma unroll
        for (int p = 0; p < PILLARS; p++) {
            float dx = t.sx - w.px[p], dy = t.sy - w.py[p];
            if (dx * dx + dy * dy < w.pr[p] * w.pr[p]) { t.slife = 0; return 0; }
        }
        float dx = t.sx - en.x, dy = t.sy - en.y;
        if (en.hp > 0.0f && dx * dx + dy * dy < TANK_R * TANK_R) { en.hp -= 1.0f; t.slife = 0; return 1; }
    }
    t.slife -= DT;
    return 0;
}

#ifdef WORLD
// A shell's flight under WORLD: 40 tiles/s is four tiles a step, past a
// tank of hit radius 0.8 in one half-step, so the path is tested whole, as
// a segment, and the first thing on it takes the shell: a wall or pillar
// stops it, the enemy within HIT_R is hit for DAMAGE, a live pillbox within
// PB_HIT is hit for DAMAGE. Returns 0 nothing, 1 the tank, 2 a pillbox,
// 3 a pillbox destroyed.
__device__ int fly_world(Tank& t, Tank& en, Arena& w) {
    if (t.slife <= 0.0f) return 0;
    float dx = t.sdx / SSPEED, dy = t.sdy / SSPEED;
    float L = fminf(SSPEED * DT, t.slife * SSPEED);
    float hit = L;
    int what = 0, box = -1;
    float tw = ray_wall(t.sx, t.sy, dx, dy);
    if (tw < hit) { hit = tw; what = 9; }
#pragma unroll
    for (int p = 0; p < PILLARS; p++) {
        float tp = ray_circle_in(t.sx, t.sy, dx, dy, w.px[p], w.py[p], w.pr[p]);
        if (tp < hit) { hit = tp; what = 9; }
    }
#ifdef FOREST
    for (int b = 0; b < w.nwall; b++) {
        float tb = ray_box(t.sx, t.sy, dx, dy, w.wx0[b], w.wy0[b], w.wx1[b], w.wy1[b]);
        if (tb < hit) { hit = tb; what = 9; }
    }
    // blocks_fire: a flat shot stops in the trees (past the half tile of a grove's edge it may skim). A
    // shell fired from deep in a grove stops where it starts, as treadcore refuses the shot outright.
#pragma unroll
    for (int g = 0; g < GROVES; g++) {
        float r = w.gr[g] - GROVE_EDGE;
        if (r <= 0.0f) continue;
        float tg = ray_circle_in(t.sx, t.sy, dx, dy, w.gx[g], w.gy[g], r);
        if (tg < hit) { hit = tg; what = 9; }
    }
#endif
    if (en.hp > 0.0f) {
        float te = ray_circle_in(t.sx, t.sy, dx, dy, en.x, en.y, HIT_R);
        if (te < hit) { hit = te; what = 1; }
    }
#pragma unroll
    for (int k = 0; k < PILLBOXES; k++) {
        if (w.bhp[k] <= 0.0f || mine(w, k, t)) continue;
        float tb = ray_circle_in(t.sx, t.sy, dx, dy, w.bx[k], w.by[k], PB_HIT);
        if (tb < hit) { hit = tb; what = 2; box = k; }
    }
    if (what == 0) {
        t.sx += dx * L; t.sy += dy * L;
        t.slife -= DT;
        return 0;
    }
    t.slife = 0.0f;
    if (what == 1) { en.hp -= DAMAGE; return 1; }
    if (what == 2) { w.bhp[box] -= DAMAGE; return w.bhp[box] <= 0.0f ? 3 : 2; }
    return 0;
}

#ifdef FOREST
// THE TAKEOVER: a tank within CAPTURE_R of a pillbox's rubble takes it; it stands again, armed in
// PB_RELOAD, its taker's. Returns 1 when tank `a` took one this step.
__device__ int takeover(Arena& w, Tank& a, Tank& b) {
    int took = 0;
    for (int k = 0; k < PILLBOXES; k++) {
        if (w.bhp[k] > 0.0f || w.bx[k] < -100.0f) continue;
        Tank* t[2] = {&a, &b};
        for (int i = 0; i < 2; i++) {
            if (t[i]->hp <= 0.0f) continue;
            float dx = t[i]->x - w.bx[k], dy = t[i]->y - w.by[k];
            if (dx * dx + dy * dy < CAPTURE_R * CAPTURE_R) {
                w.bhp[k] = PB_HEALTH; w.brl[k] = PB_RELOAD; w.bown[k] = t[i]->side;
                if (i == 0) took++;
                break;
            }
        }
    }
    return took;
}

// Pillboxes tank `a` holds, standing: the game pays SCORE_HOLD for each every ten seconds.
__device__ inline int held_by(const Arena& w, const Tank& a) {
    int n = 0;
    for (int k = 0; k < PILLBOXES; k++) n += w.bhp[k] > 0.0f && w.bown[k] == a.side;
    return n;
}
#endif

// The neutral pillboxes' guns (treadcore-sim, the pillboxes' step): each
// live one, when loaded, fires at the nearest tank within PB_RANGE with a
// clear line, hitscan, for PB_DAMAGE. Neutral: either tank. Returns how
// many struck tank `a`, the learner.
__device__ int guns(Arena& w, Tank& a, Tank& b) {
    int on_a = 0;
#pragma unroll
    for (int k = 0; k < PILLBOXES; k++) {
        if (w.bhp[k] <= 0.0f) continue;
        w.brl[k] -= DT;
        if (w.brl[k] > 0.0f) continue;
        float da = 1e9f, db = 1e9f;
        if (a.hp > 0.0f) { float dx = a.x - w.bx[k], dy = a.y - w.by[k]; da = sqrtf(dx * dx + dy * dy); }
        if (b.hp > 0.0f) { float dx = b.x - w.bx[k], dy = b.y - w.by[k]; db = sqrtf(dx * dx + dy * dy); }
        bool sa = da <= PB_RANGE && line_clear(w.bx[k], w.by[k], a.x, a.y, w) && !hidden_from(a.x, a.y, da, w) && !mine(w, k, a);
        bool sb = db <= PB_RANGE && line_clear(w.bx[k], w.by[k], b.x, b.y, w) && !hidden_from(b.x, b.y, db, w) && !mine(w, k, b);
        if (sa && (!sb || da <= db)) { a.hp -= PB_DAMAGE; w.brl[k] = PB_RELOAD; on_a++; }
        else if (sb) { b.hp -= PB_DAMAGE; w.brl[k] = PB_RELOAD; }
    }
    return on_a;
}
#endif

__device__ void place(Tank& t, Rng& r, float x0, float x1, const Arena& w) {
    for (int tries = 0; tries < 16; tries++) {
        t.x = r.in(x0, x1); t.y = r.in(4.0f, ARENA - 4.0f);
        bool ok = true;
        for (int p = 0; p < PILLARS; p++) {
            float dx = t.x - w.px[p], dy = t.y - w.py[p];
            if (dx * dx + dy * dy < (w.pr[p] + 2.0f) * (w.pr[p] + 2.0f)) ok = false;
        }
#ifdef FOREST
        for (int b = 0; b < w.nwall; b++)
            if (t.x > w.wx0[b] - 2.0f && t.x < w.wx1[b] + 2.0f && t.y > w.wy0[b] - 2.0f && t.y < w.wy1[b] + 2.0f) ok = false;
#endif
        if (ok) break;
    }
    t.a = r.in(-3.14159f, 3.14159f);
    t.v = 0; t.hp = HP; t.reload = 0; t.slife = 0; t.sx = t.sy = -100.0f; t.sdx = t.sdy = 0;
#ifdef WORLD
    t.shells = SHELLS;
#endif
}

// A duel's arena and both tanks' places, drawn from r: training's and the
// trace's, one function so the two can never draw different worlds.
__device__ inline void lay_out(Arena& w, Tank& A, Tank& B, Rng& r) {
#pragma unroll
    for (int p = 0; p < PILLARS; p++) { w.px[p] = r.in(14.0f, 50.0f); w.py[p] = r.in(6.0f, 58.0f); w.pr[p] = r.in(1.5f, 3.5f); }
#ifdef FOREST
    // The groves anywhere between the starting bands' inner edges, 2 to 4.5 tiles across the radius:
    // cover to fight from and to hide in, near enough to both starts to be used.
#pragma unroll
    for (int g = 0; g < GROVES; g++) { w.gx[g] = r.in(10.0f, 54.0f); w.gy[g] = r.in(6.0f, 58.0f); w.gr[g] = r.in(2.0f, 4.5f); }
    // Bars of wall, a tile thick and 6 to 14 long, upright or across, between the starts: something to
    // drive round, as a maze's corridors are.
    w.nwall = TRAIN_WALLS;
    for (int b = 0; b < WALLS; b++) {
        if (b >= TRAIN_WALLS) { w.wx0[b] = w.wy0[b] = 1e9f; w.wx1[b] = w.wy1[b] = -1e9f; continue; }
        float cx = r.in(18.0f, 46.0f), cy = r.in(8.0f, 56.0f), len = r.in(6.0f, 14.0f);
        bool up = r.u() < 0.5f;
        w.wx0[b] = cx - (up ? 0.5f : 0.5f * len); w.wx1[b] = cx + (up ? 0.5f : 0.5f * len);
        w.wy0[b] = cy - (up ? 0.5f * len : 0.5f); w.wy1[b] = cy + (up ? 0.5f * len : 0.5f);
    }
    for (int k = 0; k < PILLBOXES; k++) w.bown[k] = -1;
    A.side = 0; B.side = 1;
#endif
#ifdef WORLD
    // The pillboxes in the middle band, x 24 to 40: their 8 tiles reach 16
    // and 48, short of where the tanks start (x 3 to 14 and 50 to 61), so a
    // duel begins out of range and the ring is something to cross or take.
    // Clear of the pillars, and PILLBOX_SPACING (3) apart.
    for (int k = 0; k < PILLBOXES; k++) {
        for (int tries = 0; tries < 16; tries++) {
            w.bx[k] = r.in(24.0f, 40.0f); w.by[k] = r.in(6.0f, 58.0f);
            bool ok = true;
            for (int p = 0; p < PILLARS; p++) {
                float dx = w.bx[k] - w.px[p], dy = w.by[k] - w.py[p];
                if (dx * dx + dy * dy < (w.pr[p] + 1.5f) * (w.pr[p] + 1.5f)) ok = false;
            }
            for (int j = 0; j < k; j++) {
                float dx = w.bx[k] - w.bx[j], dy = w.by[k] - w.by[j];
                if (dx * dx + dy * dy < 9.0f) ok = false;
            }
#ifdef FOREST
            // A pillbox stands in the open: on a passable tile, never in the trees or a wall.
            for (int g = 0; g < GROVES; g++) {
                float dx = w.bx[k] - w.gx[g], dy = w.by[k] - w.gy[g];
                if (dx * dx + dy * dy < (w.gr[g] + 1.0f) * (w.gr[g] + 1.0f)) ok = false;
            }
            for (int b = 0; b < w.nwall; b++)
                if (w.bx[k] > w.wx0[b] - 1.5f && w.bx[k] < w.wx1[b] + 1.5f && w.by[k] > w.wy0[b] - 1.5f && w.by[k] < w.wy1[b] + 1.5f) ok = false;
#endif
            if (ok) break;
        }
        w.bhp[k] = PB_HEALTH; w.brl[k] = 0.0f;
    }
#endif
    place(A, r, 3.0f, 14.0f, w);
    place(B, r, 50.0f, 61.0f, w);
}

// ---- the gauntlet: one decision at a time, as treadcore's ---------------------------------------
//
// Paul, 6 November: "More Cuda gauntlets that match the style of the CPU gauntlet." Treadcore's gauntlet
// (scenarios/gauntlet/*.toml, LAB-REPORT-BRAIN.md section 102) puts a brain in front of one decision with
// little else on the map -- an enemy in reach, an enemy behind, a pillbox from outside its ring -- and
// counts the share of runs it passes, each file with a floor that is today's success. --gauntlet DIR
// reads files in the same words (a [[tank]], [[pillbox]], [[brain]] and [expect] each, and [[grove]] for
// the trees) and plays them here: a block a file, a thread a run, every run's dice the run's index, so
// two nets meet the same runs and their difference is paired. A seat without a [[brain]] stands idle, as
// in treadcore; a [[brain]] may be the net or one of the sparring partners' styles.
#define G_PB 4
#define G_GROVES 4
#define G_IDLE -2
#define G_NET -1
#define G_CENTRE -1
struct Scen {
    float seconds;
    float off[2][2];
    int rel[2], around[2], head_any[2], head_to[2];
    float head[2], armor[2], reload[2];
    int brain[2];
    int npb, pb_near[G_PB], pb_toward[G_PB], pb_own[G_PB];
    float pb_off[G_PB][2], pb_away[G_PB];
    int ngr, gr_near[G_GROVES], gr_toward[G_GROVES];
    float gr_off[G_GROVES][2], gr_away[G_GROVES], gr_r[G_GROVES];
    // A maze's walls: boxes about a place, [x0, x1] by [y0, y1] tiles from it (treadcore's [[terrain]]).
    int nwall, w_near[12];
    float w_box[12][4];
    // The takeover: pillboxes that must end the learner's (a bit each), and one held this long.
    int captured, hold_pb;
    float hold_s;
    int early;                        // the run may end once the ask is met (no ask that needs the whole time)
    // The expectations, -1 where a file does not ask one.
    int hits_min, hits_on_max, alive, pb_dead, pb_dmg_max, standing_max, outscores, win;
    float armor_share;
    int reaction;                     // 0 fire, 1 move
};

// A scenario played: what the judge reads at its end.
struct ScenRun { int hits, hits_on, pb_on, took; float when, held, a0, d0; };

#ifdef WORLD
// The goal a task is measured toward: its first pillbox when the file is about pillboxes, else the enemy.
__device__ inline void scen_goal(const Scen& sc, const Arena& w, const Tank* T, float& gx, float& gy) {
    bool pb = sc.npb > 0 && (sc.captured || sc.hold_pb >= 0 || sc.pb_dead || sc.standing_max >= 0 || sc.pb_dmg_max >= 0);
    gx = pb ? w.bx[0] : T[1].x; gy = pb ? w.by[0] : T[1].y;
}

// The arena and both tanks of a file, from the dice: training's tasks and the gauntlet's runs alike.
__device__ void scen_setup(const Scen& sc, Rng& r, Arena& w, Tank* T, ScenRun& run) {
    for (int p = 0; p < PILLARS; p++) { w.px[p] = -1000.0f; w.py[p] = -1000.0f; w.pr[p] = 0.01f; }
    for (int q = 0; q < PILLBOXES; q++) { w.bx[q] = -1000.0f; w.by[q] = -1000.0f; w.bhp[q] = 0.0f; w.brl[q] = 0.0f; }
#ifdef FOREST
    for (int g = 0; g < GROVES; g++) { w.gx[g] = -1000.0f; w.gy[g] = -1000.0f; w.gr[g] = 0.0f; }
    for (int b = 0; b < WALLS; b++) { w.wx0[b] = w.wy0[b] = 1e9f; w.wx1[b] = w.wy1[b] = -1e9f; }
    w.nwall = 0;
    for (int q = 0; q < PILLBOXES; q++) w.bown[q] = -1;
#endif
    const float c = 0.5f * ARENA;
    for (int i = 0; i < 2; i++) {
        float bx = c, by = c;
        if (sc.rel[i] >= 0) { bx = T[sc.rel[i]].x; by = T[sc.rel[i]].y; }
        float rot = sc.around[i] ? r.in(-3.14159265f, 3.14159265f) : 0.0f;
        float ox = sc.off[i][0] * __cosf(rot) - sc.off[i][1] * __sinf(rot), oy = sc.off[i][0] * __sinf(rot) + sc.off[i][1] * __cosf(rot);
        Tank& t = T[i];
        t.x = fminf(fmaxf(bx + ox, 1.5f), ARENA - 1.5f);
        t.y = fminf(fmaxf(by + oy, 1.5f), ARENA - 1.5f);
        t.v = 0; t.hp = sc.armor[i] * HP; t.reload = sc.reload[i]; t.slife = 0; t.sx = t.sy = -100.0f; t.sdx = t.sdy = 0; t.shells = SHELLS;
        t.a = sc.head_any[i] ? r.in(-3.14159265f, 3.14159265f) : sc.head[i];
#ifdef FOREST
        t.side = i;
#endif
    }
    for (int i = 0; i < 2; i++)
        if (sc.head_to[i] >= 0) T[i].a = atan2f(T[sc.head_to[i]].y - T[i].y, T[sc.head_to[i]].x - T[i].x);
    // A pillbox, a grove or a wall: from where it is near (the centre, a seat, an earlier pillbox), its
    // offset, then `away` tiles toward a seat (from it, when negative), as treadcore's placer does.
    auto spot = [&](int near, const float* off, int toward, float away, float& x, float& y) {
        x = c; y = c;
        if (near >= 10) { x = w.bx[near - 10]; y = w.by[near - 10]; }
        else if (near >= 0) { x = T[near].x; y = T[near].y; }
        x += off[0]; y += off[1];
        if (toward >= 0) {
            float dx = T[toward].x - x, dy = T[toward].y - y, dl = fmaxf(sqrtf(dx * dx + dy * dy), 1e-4f);
            x += dx / dl * away; y += dy / dl * away;
        }
    };
    for (int q = 0; q < sc.npb && q < PILLBOXES; q++) {
        spot(sc.pb_near[q], sc.pb_off[q], sc.pb_toward[q], sc.pb_away[q], w.bx[q], w.by[q]);
        w.bhp[q] = PB_HEALTH; w.brl[q] = 0.0f;
#ifdef FOREST
        w.bown[q] = sc.pb_own[q];
#endif
    }
#ifdef FOREST
    for (int g = 0; g < sc.ngr && g < GROVES; g++) {
        spot(sc.gr_near[g], sc.gr_off[g], sc.gr_toward[g], sc.gr_away[g], w.gx[g], w.gy[g]);
        w.gr[g] = sc.gr_r[g];
    }
    for (int b = 0; b < sc.nwall && b < WALLS; b++) {
        float z[2] = {0.0f, 0.0f}, px, py;
        spot(sc.w_near[b], z, -1, 0.0f, px, py);
        w.wx0[b] = px + sc.w_box[b][0]; w.wx1[b] = px + sc.w_box[b][1];
        w.wy0[b] = py + sc.w_box[b][2]; w.wy1[b] = py + sc.w_box[b][3];
        w.nwall = b + 1;
    }
#endif
    run.hits = run.hits_on = run.pb_on = run.took = 0;
    run.when = -1.0f; run.held = 0.0f; run.a0 = T[0].hp;
    float gx, gy;
    scen_goal(sc, w, T, gx, gy);
    run.d0 = sqrtf((T[0].x - gx) * (T[0].x - gx) + (T[0].y - gy) * (T[0].y - gy));
}

// One step of a file: the learner (seat 0) by `net`, the other seat by its file's brain.
__device__ void scen_step(const Scen& sc, const float* net, Arena& w, Tank* T, float* in, float* actA, float* actB, ScenRun& run, int t) {
    Tank& A = T[0];
    Tank& B = T[1];
    for (int i = 0; i < NOUT; i++) { actA[i] = 0.0f; actB[i] = 0.0f; }
    actA[2] = actB[2] = -1.0f;
    if (A.hp > 0.0f) decide(net, A, B, w, in, actA);
    if (B.hp > 0.0f && sc.brain[1] >= 0) scripted(B, A, Skill{0.12f, 8.0f, sc.brain[1]}, w, actB);
    float shells = A.shells;
    drive(A, actA, w);
    drive(B, actB, w);
    if (run.when < 0.0f && ((sc.reaction == 0 && A.shells < shells) || (sc.reaction == 1 && A.v > 0.5f))) run.when = t * DT;
    if (fly_world(A, B, w) == 1) run.hits++;
    if (fly_world(B, A, w) == 1) run.hits_on++;
    run.pb_on += guns(w, A, B);
#ifdef FOREST
    run.took += takeover(w, A, B);
    if (sc.hold_pb >= 0 && w.bhp[sc.hold_pb] > 0.0f && w.bown[sc.hold_pb] == 0) run.held += DT;
#endif
}

// Did the run do what its file asks?
__device__ bool scen_judge(const Scen& sc, const Arena& w, const Tank* T, const ScenRun& run) {
    const Tank& A = T[0];
    const Tank& B = T[1];
    bool ok = true;
    if (sc.hits_min >= 0) ok = ok && run.hits >= sc.hits_min;
    if (sc.hits_on_max >= 0) ok = ok && run.hits_on + run.pb_on <= sc.hits_on_max;
    if (sc.alive >= 0) ok = ok && A.hp > 0.0f;
    for (int q = 0; q < PILLBOXES; q++)
        if (sc.pb_dead & (1 << q)) ok = ok && w.bhp[q] <= 0.0f;
    if (sc.pb_dmg_max >= 0) ok = ok && run.pb_on <= sc.pb_dmg_max;
    if (sc.standing_max >= 0) {
        int st = 0;
        for (int q = 0; q < sc.npb && q < PILLBOXES; q++) st += w.bhp[q] > 0.0f;
        ok = ok && st <= sc.standing_max;
    }
    if (sc.armor_share >= 0.0f) ok = ok && A.hp >= sc.armor_share * run.a0 - 1e-3f;
    if (sc.outscores >= 0) ok = ok && run.hits > run.hits_on;
    if (sc.win >= 0) ok = ok && B.hp <= 0.0f && A.hp > 0.0f;
#ifdef FOREST
    for (int q = 0; q < PILLBOXES; q++)
        if (sc.captured & (1 << q)) ok = ok && w.bhp[q] > 0.0f && w.bown[q] == 0;
    if (sc.hold_pb >= 0) ok = ok && run.held >= sc.hold_s;
#endif
    return ok;
}

// THE TASK'S FITNESS (Paul, 6 November: "Make a better designed fitness"): what training pays a task run.
// Passing is worth TASK_PASS, more than a duel's win, because it is the decision the gauntlet will ask;
// short of passing, the run is paid for getting there -- the share of the way to its goal it closed
// (navigation, the maze's signal, which a pass alone gives too seldom to learn from) -- and for the
// game's score, as a duel is: hits given less taken, a pillbox taken, held ground.
#define TASK_PASS 3.0f
__device__ float scen_fitness(const Scen& sc, const Arena& w, const Tank* T, const ScenRun& run, bool pass) {
    float gx, gy;
    scen_goal(sc, w, T, gx, gy);
    float d1 = sqrtf((T[0].x - gx) * (T[0].x - gx) + (T[0].y - gy) * (T[0].y - gy));
    float progress = run.d0 > 1.0f ? fminf(fmaxf((run.d0 - d1) / run.d0, -1.0f), 1.0f) : 0.0f;
    float f = (pass ? TASK_PASS : 0.0f) + progress + (float)run.hits - (float)run.hits_on - 0.33f * run.pb_on;
#ifdef FOREST
    f += SCORE_CAPTURE_W * run.took + SCORE_HOLD_W * (run.held / HOLD_EVERY_S) + SCORE_KILL_W * (T[1].hp <= 0.0f ? 1.0f : 0.0f);
#endif
    return f;
}
#endif

// ---- one generation's play ----------------------------------------------------------

struct Totals {
    unsigned long long steps; // tank-steps actually played (both tanks)
    unsigned long long opened, decided; // the learner's steps where the net had its say, of its steps
    unsigned int wins, losses, draws;
    unsigned int tasks, tasks_passed;  // training's task runs, and those that passed
#ifdef WORLD
    // The learner's: pillbox shots that struck it, pillboxes it destroyed,
    // shells it fired, and of those the ones that struck a tank or a pillbox.
    unsigned long long pill_struck, pill_killed, shots, hits;
#endif
};

// Block b plays individual b / reps (reps blocks to an individual, for the
// yardstick) for blockDim.x duels. opp_mode 0, training: with chance
// `spar` a sparring partner of drawn skill, with chance `lad` a milestone
// of the ladder, else an opponent from the hall of fame, half the time one
// of the newest 16. 1: the yardstick. 2: the ladder's milestone `rung`.
__global__ void __launch_bounds__(256, MINB) play(const float* __restrict__ pop, const float* __restrict__ hall, int hall_n, int hall_cap, int hall_next,
                     const float* __restrict__ ladder, int ladder_n, float lad, int rung, float drawv,
                     int reps, int opp_mode, float spar, int steps, uint32_t seed, int gen, float* fitness, Totals* tot,
                     const Scen* __restrict__ tasks, int ntasks, float task_p) {
    extern __shared__ float sm[];
    float* wA = sm;
    float* wB = sm + NW;
    __shared__ float score;
    __shared__ unsigned int sw, sl, sd;
#ifdef WORLD
    __shared__ unsigned int s_ps, s_pk, s_sh, s_hi;
#endif
    int ind = blockIdx.x / reps;
    Rng br{mix(mix(seed, gen), blockIdx.x)};
    Skill skill = YARDSTICK;
    const float* opp = nullptr;
    if (opp_mode == 0) {
        float u = br.u();
        if (u < spar) {
            skill = Skill{br.in(0.05f, 0.6f), br.in(SPAR_NEAR, SPAR_FAR), 0};
#ifdef WORLD
            skill.style = min((int)(br.u() * STYLES), STYLES - 1);
#endif
        }
        else if (u < spar + lad && ladder_n > 0) opp = ladder + (size_t)min((int)(br.u() * ladder_n), ladder_n - 1) * NW;
        else {
            bool recent = br.u() < 0.5f;
            int k = (int)(br.u() * (recent ? min(hall_n, 16) : hall_n));
            opp = hall + (size_t)(recent ? (hall_next - 1 - k + hall_cap) % hall_cap : k) * NW;
        }
    } else if (opp_mode == 2) opp = ladder + (size_t)rung * NW;
    bool net_opp = opp != nullptr;
    for (int i = threadIdx.x; i < NW; i += blockDim.x) {
        wA[i] = pop[(size_t)ind * NW + i];
        if (net_opp) wB[i] = opp[i];
    }
    if (threadIdx.x == 0) { score = 0; sw = sl = sd = 0; }
#ifdef WORLD
    if (threadIdx.x == 0) { s_ps = s_pk = s_sh = s_hi = 0; }
#endif
    __syncthreads();

    Rng r{mix(mix(mix(seed, gen), blockIdx.x), threadIdx.x + 1)};
    Arena w;
    Tank A, B;
    lay_out(w, A, B, r);
    // A TASK IN PLACE OF A DUEL (Paul, 6 November: "more training aspects ... More tests can be added"):
    // with chance task_p, in training only, this thread plays one of the gauntlet's families -- a maze,
    // a pillbox to take, a base to hold, the trees -- on dice of training's own (never the gauntlet's
    // seed, so the gauntlet's runs stay unseen), paid by scen_fitness. The champion is then chosen on
    // the decisions the gauntlet will ask, not only on duels.
    bool task = false;
#ifdef WORLD
    // WHOLE WARPS, AND THE SAME TASKS FOR EVERY INDIVIDUAL: the first task_p of a block's warps play
    // tasks (with 128 duels and 0.25, one warp of 32), each warp the task the generation's dice give it,
    // the same for every individual of the generation, so their fitnesses compare like with like and no
    // warp waits on a 150-second maze for one thread. The tasks rotate with the generations.
    int warps = blockDim.x >> 5, task_warps = (int)(task_p * warps + 0.5f), wi = threadIdx.x >> 5;
    if (opp_mode == 0 && ntasks > 0 && wi < task_warps) {
        task = true;
        Rng tr{mix(mix(seed ^ 0x7a5c0000u, gen), (uint32_t)wi)};
        const Scen& sc = tasks[min((int)(tr.u() * ntasks), ntasks - 1)];
        Tank T[2];
        ScenRun run;
        scen_setup(sc, r, w, T, run);
        float tin[NIN], ta[NOUT], tb[NOUT];
        int st = (int)(sc.seconds / DT + 0.5f), t = 0;
        for (; t < st && T[0].hp > 0.0f; t++) {
            scen_step(sc, wA, w, T, tin, ta, tb, run, t);
            if (sc.early && t % 5 == 4 && scen_judge(sc, w, T, run)) break;
        }
        bool ok = scen_judge(sc, w, T, run);
        atomicAdd(&score, scen_fitness(sc, w, T, run, ok));
        atomicAdd(&tot->steps, (unsigned long long)(2 * t));
        atomicAdd(&tot->tasks, 1u);
        if (ok) atomicAdd(&tot->tasks_passed, 1u);
    }
#endif
    if (!task) {

    float in[NIN], actA[NOUT], actB[NOUT];
    float aim = 0;
    unsigned int opened = 0;
#ifdef WORLD
    unsigned int a_hit_tank = 0, a_hit_box = 0, a_killed = 0, a_struck = 0;
#endif
#ifdef FOREST
    unsigned int a_took = 0;
    float held_s = 0.0f;
#endif
    int t = 0;
    for (; t < steps; t++) {
        if (A.hp <= 0.0f || B.hp <= 0.0f) break;
        opened += decide(wA, A, B, w, in, actA);
        aim += in[29] > 0.0f ? in[29] * (in[28] * in[28] < 0.01f ? 1.0f : 0.2f) : 0.0f;
        if (net_opp) decide(wB, B, A, w, in, actB);
        else scripted(B, A, skill, w, actB);
        drive(A, actA, w);
        drive(B, actB, w);
#ifdef WORLD
        int fa = fly_world(A, B, w);
        fly_world(B, A, w);
        if (fa == 1) a_hit_tank++;
        if (fa >= 2) a_hit_box++;
        if (fa == 3) a_killed++;
        a_struck += guns(w, A, B);
#ifdef FOREST
        a_took += takeover(w, A, B);
        held_s += held_by(w, A) * DT;
#endif
#else
        fly(A, B, w);
        fly(B, A, w);
#endif
    }
#ifdef WORLD
    // In hits, as before: what the learner's own shells did to the enemy (a
    // pillbox's work on it is not the learner's), and all it lost, the
    // pillboxes' included.
    float dealt = (float)a_hit_tank, taken = (HP - fmaxf(A.hp, 0.0f)) / DAMAGE;
#else
    float dealt = HP - fmaxf(B.hp, 0.0f), taken = HP - fmaxf(A.hp, 0.0f);
#endif
    bool won = B.hp <= 0.0f && A.hp > 0.0f, lost = A.hp <= 0.0f && B.hp > 0.0f;
    // A draw is worth `drawv` (--draw): at 0 a net that waits for an
    // attacker loses nothing by never finishing a duel, and two such nets
    // drew all 2,048 duels of the first exam (28 October); below 0 it pays
    // to finish the fight.
    bool drew = !won && !lost;
    float f = dealt - taken + (won ? 2.0f : 0.0f) - (lost ? 2.0f : 0.0f) + (drew ? drawv : 0.0f) + 0.5f * aim / steps;
#ifdef FOREST
    // The game's score of the duel, in kills: the kill, each pillbox taken, each held ten seconds.
    f += SCORE_KILL_W * (B.hp <= 0.0f ? 1.0f : 0.0f) + SCORE_CAPTURE_W * a_took + SCORE_HOLD_W * (held_s / HOLD_EVERY_S);
#endif
    atomicAdd(&score, f);
    if (won) atomicAdd(&sw, 1u); else if (lost) atomicAdd(&sl, 1u); else atomicAdd(&sd, 1u);
    atomicAdd(&tot->steps, (unsigned long long)(2 * t));
    atomicAdd(&tot->opened, (unsigned long long)opened);
    atomicAdd(&tot->decided, (unsigned long long)t);
#ifdef WORLD
    atomicAdd(&s_ps, a_struck);
    atomicAdd(&s_pk, a_killed);
    atomicAdd(&s_sh, (unsigned int)(SHELLS - A.shells + 0.5f));
    atomicAdd(&s_hi, a_hit_tank + a_hit_box);
#endif
    }
    __syncthreads();
    if (threadIdx.x == 0) {
        atomicAdd(&fitness[ind], score / (blockDim.x * reps));
        atomicAdd(&tot->wins, sw); atomicAdd(&tot->losses, sl); atomicAdd(&tot->draws, sd);
#ifdef WORLD
        atomicAdd(&tot->pill_struck, (unsigned long long)s_ps);
        atomicAdd(&tot->pill_killed, (unsigned long long)s_pk);
        atomicAdd(&tot->shots, (unsigned long long)s_sh);
        atomicAdd(&tot->hits, (unsigned long long)s_hi);
#endif
    }
}

#ifdef WORLD
// ---- the trace: one champion's duels, every step of every one -----------------------------
//
// Paul, 4 November: "a montage of kill shots and pillbox fights ... watch the
// multitudes of permutations in the same world such that the camera following
// the best winning tank as it grows via generations ... All the other
// permutations are ghosts." --trace plays each champion named, a block of
// --trace-duels duels, IN ONE ARENA FROM ONE START: the pillars, the pillboxes
// and both tanks' places drawn once from --trace-seed. The dice choose each
// duel's opponent as training's do, a sparring partner of drawn skill or a
// milestone, and the dice are the duel's index, not the champion's: duel d is
// the same contest in every chapter, so a montage compares like with like.
// Nothing here is training's: play() is untouched, and the arena is lay_out(),
// training's own. Each step of each duel is TREC floats: both tanks (x, y,
// heading, armour, shell x, shell y, shell life), the pillboxes' health, what
// each tank's shell did that step (fly_world's 0 to 3), and how many pillbox
// shots struck each tank. Version 2 (Paul, 4 November: "show all the inputs,
// transform and output ... the activation of the circuits"): each step also
// TNET floats of the champion's mind, the NIN numbers its net read and the
// NOUT it drove with, so the film can draw the net thinking; the hidden
// layers are recomputed from its weights, the forward pass being exact.
#define TREC 22
#define TNET (NIN + NOUT)
// The arena as the trace writes it: pillars' x, y, r, pillboxes' x, y, and under FOREST the groves' x, y,
// r. A forest trace is version 3, and says how many groves after the net's shape.
#ifdef FOREST
#define ARENA_REC (3 * PILLARS + 2 * PILLBOXES + 3 * GROVES + 4 * WALLS)
#define TRACE_VERSION 3
#else
#define ARENA_REC (3 * PILLARS + 2 * PILLBOXES)
#define TRACE_VERSION 2
#endif
//
// THE SPREAD (Paul, 5 November: "can you show more ghosts in the movie"). One start and eleven kinds of
// opponent made 128 duels only eight to twelve games: the nets are deterministic, and a duel with the
// same opponent from the same place played out the same, so the film's ghosts stood on one another.
// --trace-spread S moves each duel's two tanks up to S tiles from the start and turns them up to a fifth
// of a turn, by dice that are the duel's index (a stream apart from the opponent's, so duel d keeps its
// opponent): the arena, the pillars and the pillboxes are one world still, duel d is still the same
// contest in every chapter, and the permutations are permutations. A moved tank is kept off the pillars
// as place() keeps it, and inside its own starting band; 0 is the one start of before.
__global__ void __launch_bounds__(256, 1) trace_play(const float* __restrict__ champ, const float* __restrict__ opps, int n_opps, int steps,
                                                     uint32_t seed, float drawv, float spread, float* rec, int* outcome, int* lasted, float* fit,
                                                     int* opp_of, float* arena_out, float* mind) {
    int d = threadIdx.x;
    Rng r{mix(seed, 0x7a11u)};
    Arena w;
    Tank A, B;
    lay_out(w, A, B, r);
    Rng dice{mix(seed, (uint32_t)d + 1u)};
    Skill skill = YARDSTICK;
    const float* opp = nullptr;
    int oi = -1;
    if (dice.u() >= 0.5f && n_opps > 0) { oi = min((int)(dice.u() * n_opps), n_opps - 1); opp = opps + (size_t)oi * NW; }
    else {
        skill = Skill{dice.in(0.05f, 0.6f), dice.in(SPAR_NEAR, SPAR_FAR), 0};
        skill.style = min((int)(dice.u() * STYLES), STYLES - 1);
        oi = -1 - skill.style;
    }
    opp_of[d] = oi;
    if (spread > 0.0f) {
        Rng jit{mix(seed, 0x51ed0000u + (uint32_t)d)};
        Tank* ts[2] = {&A, &B};
        const float band[2][2] = {{3.0f, 14.0f}, {50.0f, 61.0f}};
        for (int i = 0; i < 2; i++) {
            Tank& t = *ts[i];
            float x = fminf(fmaxf(t.x + jit.in(-spread, spread), band[i][0]), band[i][1]);
            float y = fminf(fmaxf(t.y + jit.in(-spread, spread), 4.0f), ARENA - 4.0f);
            bool ok = true;
            for (int p = 0; p < PILLARS; p++) {
                float dx = x - w.px[p], dy = y - w.py[p];
                if (dx * dx + dy * dy < (w.pr[p] + 2.0f) * (w.pr[p] + 2.0f)) ok = false;
            }
            if (ok) { t.x = x; t.y = y; }
            t.a += jit.in(-1.25f, 1.25f);
        }
    }
    if (d == 0) {
        for (int p = 0; p < PILLARS; p++) { arena_out[p] = w.px[p]; arena_out[PILLARS + p] = w.py[p]; arena_out[2 * PILLARS + p] = w.pr[p]; }
        for (int k = 0; k < PILLBOXES; k++) { arena_out[3 * PILLARS + k] = w.bx[k]; arena_out[3 * PILLARS + PILLBOXES + k] = w.by[k]; }
#ifdef FOREST
        for (int g = 0; g < GROVES; g++) {
            arena_out[3 * PILLARS + 2 * PILLBOXES + g] = w.gx[g];
            arena_out[3 * PILLARS + 2 * PILLBOXES + GROVES + g] = w.gy[g];
            arena_out[3 * PILLARS + 2 * PILLBOXES + 2 * GROVES + g] = w.gr[g];
        }
        // The walls, x0 y0 x1 y1 each; one not laid is x0 > x1.
        for (int b = 0; b < WALLS; b++) {
            float* o = arena_out + 3 * PILLARS + 2 * PILLBOXES + 3 * GROVES + 4 * b;
            o[0] = w.wx0[b]; o[1] = w.wy0[b]; o[2] = w.wx1[b]; o[3] = w.wy1[b];
        }
#endif
    }
    float in[NIN], actA[NOUT], actB[NOUT];
    float aim = 0;
    unsigned int hit_tank = 0;
    float* out = rec + (size_t)d * steps * TREC;
    float* thought = mind + (size_t)d * steps * TNET;
    int t = 0;
    for (; t < steps; t++) {
        if (A.hp <= 0.0f || B.hp <= 0.0f) break;
        decide(champ, A, B, w, in, actA);
        aim += in[29] > 0.0f ? in[29] * (in[28] * in[28] < 0.01f ? 1.0f : 0.2f) : 0.0f;
        // The mind before the opponent's turn reuses `in`.
        float* m = thought + (size_t)t * TNET;
        for (int i = 0; i < NIN; i++) m[i] = in[i];
        for (int k = 0; k < NOUT; k++) m[NIN + k] = actA[k];
        if (opp) decide(opp, B, A, w, in, actB);
        else scripted(B, A, skill, w, actB);
        drive(A, actA, w);
        drive(B, actB, w);
        int fa = fly_world(A, B, w);
        int fb = fly_world(B, A, w);
        float bh = B.hp;
        int pa = guns(w, A, B);
#ifdef FOREST
        takeover(w, A, B);
#endif
        int pb = (int)((bh - B.hp) / PB_DAMAGE + 0.5f);
        if (fa == 1) hit_tank++;
        float* o = out + (size_t)t * TREC;
        o[0] = A.x; o[1] = A.y; o[2] = A.a; o[3] = A.hp; o[4] = A.sx; o[5] = A.sy; o[6] = A.slife;
        o[7] = B.x; o[8] = B.y; o[9] = B.a; o[10] = B.hp; o[11] = B.sx; o[12] = B.sy; o[13] = B.slife;
        for (int k = 0; k < PILLBOXES; k++) o[14 + k] = w.bhp[k];
        o[18] = (float)fa; o[19] = (float)fb; o[20] = (float)pa; o[21] = (float)pb;
    }
    bool won = B.hp <= 0.0f && A.hp > 0.0f, lost = A.hp <= 0.0f && B.hp > 0.0f, drew = !won && !lost;
    float dealt = (float)hit_tank, taken = (HP - fmaxf(A.hp, 0.0f)) / DAMAGE;
    lasted[d] = t;
    outcome[d] = won ? 1 : lost ? -1 : 0;
    fit[d] = dealt - taken + (won ? 2.0f : 0.0f) - (lost ? 2.0f : 0.0f) + (drew ? drawv : 0.0f) + 0.5f * aim / steps;
}

__global__ void __launch_bounds__(256, 1) gauntlet_play(const float* __restrict__ net, const Scen* __restrict__ scen, int runs, uint32_t seed,
                                                        unsigned int* passed, float* react, unsigned char* pass_of) {
    extern __shared__ float wsh[];
    for (int i = threadIdx.x; i < NW; i += blockDim.x) wsh[i] = net[i];
    __syncthreads();
    const Scen& sc = scen[blockIdx.x];
    for (int k = threadIdx.x; k < runs; k += blockDim.x) {
        Rng r{mix(mix(seed, 0x6a17u + blockIdx.x), (uint32_t)k + 1u)};
        Arena w;
        Tank T[2];
        ScenRun run;
        scen_setup(sc, r, w, T, run);
        float in[NIN], actA[NOUT], actB[NOUT];
        int steps = (int)(sc.seconds / DT + 0.5f);
        for (int t = 0; t < steps && T[0].hp > 0.0f; t++) {
            scen_step(sc, wsh, w, T, in, actA, actB, run, t);
            if (sc.early && t % 5 == 4 && scen_judge(sc, w, T, run)) break;
        }
        bool ok = scen_judge(sc, w, T, run);
        if (ok) atomicAdd(&passed[blockIdx.x], 1u);
        react[(size_t)blockIdx.x * runs + k] = run.when;
        pass_of[(size_t)blockIdx.x * runs + k] = ok ? 1 : 0;
    }
}
#endif

// ---- the next generation ----------------------------------------------------------------

// The parent of every child: the elite keep their place, the rest are drawn
// from the top quarter of the ranking.
__global__ void choose(const int* rank, int n, int elite, uint32_t seed, int gen, int* parent) {
    int c = blockIdx.x * blockDim.x + threadIdx.x;
    if (c >= n) return;
    if (c < elite) { parent[c] = rank[c]; return; }
    int top = max(n / 4, 1);
    parent[c] = rank[hash32(mix(mix(seed ^ 0x5bd1e995u, gen), c)) % top];
}

// Every weight of every child: its parent's, and for all but the elite a
// gaussian nudge. One thread a weight.
__global__ void breed(const float* __restrict__ pop, const int* parent, int n, int elite, float sigma, uint32_t seed, int gen, float* next) {
    size_t i = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= (size_t)n * NW) return;
    int c = (int)(i / NW), j = (int)(i % NW);
    float v = pop[(size_t)parent[c] * NW + j];
    if (c >= elite) v += sigma * gauss(mix(mix(mix(seed, gen), c), j));
    next[i] = v;
}

__global__ void seed_pop(float* pop, size_t n, uint32_t seed) {
    size_t i = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
#ifdef HYBRID
    // Beginning at the rules: small corrections, and every gate shut (the last number
    // is the confidence's bias).
    pop[i] = (i % NW == NW - 1) ? -0.5f : 0.03f * gauss(mix(seed, (uint32_t)i));
#else
    pop[i] = 0.3f * gauss(mix(seed, (uint32_t)i));
#endif
}

// ---- the host: setup, the loop, the numbers ---------------------------------------------

struct Opt {
    int pop = 8192, eps = 256, steps = STEPS_DEFAULT, gens = 100, elite = 16, every = 5, cap = 4096, yard = 10, yard_eps = 4096;
    float sigma = 0.03f, spar = 0.5f, lad = 0.2f, draw = -1.0f;
    int exam_eps = 2048;
    uint32_t seed = 1;
    std::string out = "", save = "", load = "", exp = "", imp = "", ladder = "", exam = "";
    std::string trace = "", champs = "", opps = "";
    int trace_duels = 64;
    uint32_t trace_seed = 7;
    float trace_spread = 0.0f;
    std::string gauntlet = "", gauntlet_out = "", gauntlet_ref = "";
    int gauntlet_runs = 256;
    std::string tasks = "";
    float task_p = 0.25f;
    uint32_t gauntlet_seed = 0x6a75u;
    bool bench = false;
};

static void usage() {
    fprintf(stderr,
            "crucible [--pop N] [--eps N] [--steps N] [--gens N] [--elite N] [--every K] [--cap N]\n"
            "         [--sigma S] [--spar P] [--seed N] [--yard K] [--out FILE.tsv] [--save FILE] [--load FILE] [--bench]\n"
            "         [--export CHAMP.bin] [--import A.bin,B.bin,...] [--ladder M1.bin,M2.bin,...] [--lad P]\n"
            "         [--exam FILE.tsv] [--exam-eps N] [--draw V]\n"
            "crucible --trace OUT.bin --champions C1.bin,C2.bin,... [--opponents M1.bin,...] [--trace-duels N] [--trace-seed S] [--trace-spread TILES]\n"
            "crucible --gauntlet DIR --champions C1.bin,... [--gauntlet-runs N] [--gauntlet-out FILE.tsv]\n"
            "  --tasks DIR [--task-p P]: the first P of each individual's warps play one of DIR's files instead of duels,\n"
            "    on training's own dice, paid for passing, for the way it closed to its goal and for the score;\n"
            "  with training: --gauntlet DIR [--gauntlet-ref NEWEST.bin] [--gauntlet-out FILE.tsv] reads the final champion\n"
            "    (and the reference on the same runs) through every file of DIR, treadcore's gauntlet's words;\n"
            "  --export writes the last generation's champion (its weights, as floats);\n"
            "  --import puts other islands' champions into the hall of fame first;\n"
            "  --ladder loads milestones (oldest first): opponents with chance --lad, and a yardstick;\n"
            "  --exam writes the final champion's score against each milestone and the yardstick;\n"
            "  --draw is a drawn duel's worth in training (a win is +2, a loss -2, besides damage).\n"
            "  A state or champion of a smaller net is widened into this one, every new weight zero.\n"
            "  --trace (WORLD builds) plays each champion's N duels in one arena from one start, the\n"
            "    dice choosing each duel's opponent, and writes every step of every duel (montage.py).\n");
    exit(2);
}

// ---- the net's shape, and widening ------------------------------------------------------
//
// A state or a champion says how many weights it holds; a v3 state also says
// its shape. The shapes this program builds: 32 in and 3 out (plain), 35 and
// 4 (HYBRID), 52 and 3 (WORLD), 55 and 4 (both), each at any H.

#if defined(WORLD) && defined(HYBRID)
#define KIND "world hybrid"
#elif defined(WORLD)
#define KIND "world"
#elif defined(HYBRID)
#define KIND "hybrid"
#else
#define KIND "plain"
#endif

struct Dims { int nin, h, nout; };
static const Dims HERE = {NIN, H, NOUT};
static long nw_of(Dims d) { return (long)d.nin * d.h + d.h + (long)d.h * d.h + d.h + (long)d.h * d.nout + d.nout; }
static bool same(Dims a, Dims b) { return a.nin == b.nin && a.h == b.h && a.nout == b.nout; }
static std::string shape(Dims d) {
    char b[48];
    snprintf(b, sizeof b, "%d-%d-%d-%d", d.nin, d.h, d.h, d.nout);
    return b;
}

// The shape of `nw` weights: this build's own first, then each kind at the
// H that fits. False when none does.
static bool dims_of(long nw, Dims* out) {
    if (nw == nw_of(HERE)) { *out = HERE; return true; }
    static const int kinds[4][2] = {{32, 3}, {35, 4}, {52, 3}, {55, 4}};
    for (auto& k : kinds)
        for (int h = 1; h <= 512; h++) {
            Dims d = {k[0], h, k[1]};
            long n = nw_of(d);
            if (n == nw) { *out = d; return true; }
            if (n > nw) break;
        }
    return false;
}

// Where an input of shape `s` goes in this build: the first 32 stay, a
// hybrid's rules stay at 32..34, the world's twenty move to AT_WORLD.
// -1: it has nowhere to go (a hybrid's rules into a plain net, the world's
// senses into a net without them), and the widening is refused.
static int input_to(int i, Dims s) {
    if (i < 32) return i;
    bool s_rules = s.nin == 35 || s.nin == 55;
    if (s_rules && i < 35) return RULES_IN ? i : -1;
    int at = s_rules ? 35 : 32;
    return WORLD_IN ? AT_WORLD + (i - at) : -1;
}

// One net of shape `s` into this build's layout, every new weight zero: a
// new input reaches nothing, a new hidden unit is tanh(0) = 0 and reaches
// nothing, so the old net's answers come out unchanged. A new output
// starts at 0, except a hybrid's confidence, which starts shut (-0.5, as
// seed_pop's): a plain net brought into a hybrid begins at the rules.
// Returns a sentence when it cannot be done, else null.
static const char* widen(const float* src, Dims s, float* dst) {
    if (s.nin > NIN || s.h > H || s.nout > NOUT) return "a larger net never shrinks into a smaller one: its weights would be lost";
    for (int i = 0; i < s.nin; i++)
        if (input_to(i, s) < 0) return "its inputs have no place in this net (a hybrid's rules, or the world's senses, would be dropped)";
    const float *sW1 = src, *sb1 = sW1 + s.nin * s.h, *sW2 = sb1 + s.h, *sb2 = sW2 + s.h * s.h, *sW3 = sb2 + s.h, *sb3 = sW3 + s.h * s.nout;
    float *W1 = dst, *b1 = W1 + NIN * H, *W2 = b1 + H, *b2 = W2 + H * H, *W3 = b2 + H, *b3 = W3 + H * NOUT;
    std::fill(dst, dst + NW, 0.0f);
    for (int j = 0; j < s.h; j++) {
        for (int i = 0; i < s.nin; i++) W1[j * NIN + input_to(i, s)] = sW1[j * s.nin + i];
        b1[j] = sb1[j];
        for (int i = 0; i < s.h; i++) W2[j * H + i] = sW2[j * s.h + i];
        b2[j] = sb2[j];
    }
    for (int k = 0; k < s.nout; k++) {
        for (int i = 0; i < s.h; i++) W3[k * H + i] = sW3[k * s.h + i];
        b3[k] = sb3[k];
    }
#ifdef HYBRID
    if (s.nout < NOUT) b3[NOUT - 1] = -0.5f;
#endif
    return nullptr;
}

// A champion's file (raw floats, no header), widened when it is of a
// smaller net. False, with the old sentence, when it is of no shape here.
static bool read_champion(const std::string& path, std::vector<float>& w, std::string* note) {
    FILE* f = fopen(path.c_str(), "rb");
    if (!f) return false;
    fseek(f, 0, SEEK_END);
    long bytes = ftell(f);
    fseek(f, 0, SEEK_SET);
    Dims s;
    if (bytes <= 0 || bytes % 4 || !dims_of(bytes / 4, &s)) { fclose(f); return false; }
    std::vector<float> raw(bytes / 4);
    bool ok = fread(raw.data(), 4, raw.size(), f) == raw.size();
    fclose(f);
    if (!ok) return false;
    w.assign(NW, 0.0f);
    if (same(s, HERE)) { w = raw; return true; }
    if (widen(raw.data(), s, w.data())) return false;
    *note = "widened from " + shape(s) + " to " + shape(HERE);
    return true;
}

static std::vector<std::string> split(const std::string& s) {
    std::vector<std::string> v;
    for (size_t a = 0; a <= s.size();) {
        size_t b = s.find(',', a);
        if (b == std::string::npos) b = s.size();
        if (b > a) v.push_back(s.substr(a, b - a));
        a = b + 1;
    }
    return v;
}

// --trace: the file montage.py reads. A header of ints -- the mark 0x63747231
// ("ctr1"), the version, champions, duels, steps, TREC, PILLARS, PILLBOXES, NW
// -- then floats DT, ARENA, HP, PB_HEALTH, GUN_RANGE, PB_RANGE; the arena
// (pillars' x, y, r, pillboxes' x, y); each duel's opponent (-1 - style a
// sparring partner of that style, 0 the script to 4 Timid; else the --opponents index); then for each champion its duels'
// outcomes (1 won, -1 lost, 0 drawn), steps played and fitness, and every
// step of every duel, TREC floats. A duel that ends early stops there: its
// later steps are zero, and `steps played` says where. Version 2: after the
// floats, the net's NIN, H and NOUT as ints, and after each champion's steps
// its mind, TNET floats a step of every duel.
static int run_trace(const Opt& o) {
#ifndef WORLD
    fprintf(stderr, "--trace is for the WORLD build: make WORLD=1\n");
    return 2;
#else
    std::vector<std::string> cs = split(o.champs), os = split(o.opps);
    int nd = o.trace_duels;
    if (cs.empty() || nd < 1 || nd > 256) { fprintf(stderr, "--trace needs --champions, and 1 to 256 --trace-duels\n"); return 2; }
    std::vector<float> champs, opps;
    for (auto& path : cs) {
        std::vector<float> w; std::string note;
        if (!read_champion(path, w, &note)) { fprintf(stderr, "%s: not a champion of this net\n", path.c_str()); return 1; }
        champs.insert(champs.end(), w.begin(), w.end());
    }
    for (auto& path : os) {
        std::vector<float> w; std::string note;
        if (!read_champion(path, w, &note)) { fprintf(stderr, "%s: not a champion of this net\n", path.c_str()); return 1; }
        opps.insert(opps.end(), w.begin(), w.end());
    }
    size_t recn = (size_t)nd * o.steps * TREC, mindn = (size_t)nd * o.steps * TNET;
    float *d_ch, *d_op = nullptr, *d_rec, *d_fit, *d_ar, *d_mind;
    int *d_out, *d_last, *d_opp;
    CK(cudaMalloc(&d_ch, champs.size() * sizeof(float)));
    CK(cudaMemcpy(d_ch, champs.data(), champs.size() * sizeof(float), cudaMemcpyHostToDevice));
    if (!opps.empty()) {
        CK(cudaMalloc(&d_op, opps.size() * sizeof(float)));
        CK(cudaMemcpy(d_op, opps.data(), opps.size() * sizeof(float), cudaMemcpyHostToDevice));
    }
    CK(cudaMalloc(&d_rec, recn * sizeof(float)));
    CK(cudaMalloc(&d_mind, mindn * sizeof(float)));
    CK(cudaMalloc(&d_fit, nd * sizeof(float)));
    CK(cudaMalloc(&d_ar, ARENA_REC * sizeof(float)));
    CK(cudaMalloc(&d_out, nd * sizeof(int)));
    CK(cudaMalloc(&d_last, nd * sizeof(int)));
    CK(cudaMalloc(&d_opp, nd * sizeof(int)));
    FILE* f = fopen(o.trace.c_str(), "wb");
    if (!f) { perror(o.trace.c_str()); return 1; }
    int hdr[9] = {0x63747231, TRACE_VERSION, (int)cs.size(), nd, o.steps, TREC, PILLARS, PILLBOXES, NW};
    float con[6] = {DT, ARENA, HP, PB_HEALTH, GUN_RANGE, PB_RANGE};
    int shape[3] = {NIN, H, NOUT};
    fwrite(hdr, sizeof(int), 9, f);
    fwrite(con, sizeof(float), 6, f);
    fwrite(shape, sizeof(int), 3, f);
#ifdef FOREST
    int more[2] = {GROVES, WALLS};
    fwrite(more, sizeof(int), 2, f);
#endif
    std::vector<float> rec(recn), mind(mindn), fit(nd), ar(ARENA_REC);
    std::vector<int> out(nd), last(nd), opp(nd);
    for (size_t c = 0; c < cs.size(); c++) {
        CK(cudaMemset(d_rec, 0, recn * sizeof(float)));
        CK(cudaMemset(d_mind, 0, mindn * sizeof(float)));
        trace_play<<<1, nd>>>(d_ch + c * NW, d_op, (int)os.size(), o.steps, o.trace_seed, o.draw, o.trace_spread, d_rec, d_out, d_last, d_fit, d_opp, d_ar, d_mind);
        CK(cudaGetLastError());
        CK(cudaDeviceSynchronize());
        CK(cudaMemcpy(rec.data(), d_rec, recn * sizeof(float), cudaMemcpyDeviceToHost));
        CK(cudaMemcpy(mind.data(), d_mind, mindn * sizeof(float), cudaMemcpyDeviceToHost));
        CK(cudaMemcpy(fit.data(), d_fit, nd * sizeof(float), cudaMemcpyDeviceToHost));
        CK(cudaMemcpy(out.data(), d_out, nd * sizeof(int), cudaMemcpyDeviceToHost));
        CK(cudaMemcpy(last.data(), d_last, nd * sizeof(int), cudaMemcpyDeviceToHost));
        if (c == 0) {
            CK(cudaMemcpy(ar.data(), d_ar, ar.size() * sizeof(float), cudaMemcpyDeviceToHost));
            CK(cudaMemcpy(opp.data(), d_opp, nd * sizeof(int), cudaMemcpyDeviceToHost));
            fwrite(ar.data(), sizeof(float), ar.size(), f);
            fwrite(opp.data(), sizeof(int), nd, f);
        }
        fwrite(out.data(), sizeof(int), nd, f);
        fwrite(last.data(), sizeof(int), nd, f);
        fwrite(fit.data(), sizeof(float), nd, f);
        fwrite(rec.data(), sizeof(float), recn, f);
        fwrite(mind.data(), sizeof(float), mindn, f);
        int w = 0, l = 0;
        for (int d = 0; d < nd; d++) { w += out[d] == 1; l += out[d] == -1; }
        printf("# trace %s: %d duels, %d won, %d lost, %d drawn\n", cs[c].c_str(), nd, w, l, nd - w - l);
    }
    fclose(f);
    return 0;
#endif
}


#ifdef WORLD
#ifdef WORLD
// ---- the gauntlet's files ------------------------------------------------------------------------
//
// The words of treadcore's scenarios/gauntlet/*.toml, as much of TOML as they use: comments, `key =
// value` (a number, a "string", true or false, [a, b], or an inline { k = v, ... }), [table] and
// [[array of tables]]. A file this arena cannot play -- a key it does not know, or trees without a
// FOREST build -- is said and left out, never half played.
struct TomlT { std::string kind; std::map<std::string, std::string> kv; };

static std::string trim(const std::string& s) {
    size_t a = s.find_first_not_of(" \t\r\n"), b = s.find_last_not_of(" \t\r\n");
    return a == std::string::npos ? "" : s.substr(a, b - a + 1);
}
static std::string unq(const std::string& v) {
    std::string t = trim(v);
    return t.size() >= 2 && t.front() == '"' && t.back() == '"' ? t.substr(1, t.size() - 2) : t;
}
static std::vector<TomlT> toml_read(const std::string& path) {
    std::vector<TomlT> out(1);
    FILE* f = fopen(path.c_str(), "r");
    if (!f) return {};
    char buf[4096];
    while (fgets(buf, sizeof buf, f)) {
        std::string l = buf;
        bool q = false;
        for (size_t i = 0; i < l.size(); i++) {
            if (l[i] == '"') q = !q;
            if (l[i] == '#' && !q) { l = l.substr(0, i); break; }
        }
        l = trim(l);
        if (l.empty()) continue;
        if (l.rfind("[[", 0) == 0) { out.push_back({trim(l.substr(2, l.find("]]") - 2)), {}}); continue; }
        if (l[0] == '[') { out.push_back({trim(l.substr(1, l.find(']') - 1)), {}}); continue; }
        size_t e = l.find('=');
        if (e != std::string::npos) out.back().kv[trim(l.substr(0, e))] = trim(l.substr(e + 1));
    }
    fclose(f);
    return out;
}
static std::map<std::string, std::string> inline_table(const std::string& v) {
    std::map<std::string, std::string> m;
    std::string t = trim(v);
    if (t.size() < 2 || t.front() != '{') return m;
    t = t.substr(1, t.size() - 2);
    int depth = 0;
    size_t a = 0;
    for (size_t i = 0; i <= t.size(); i++) {
        if (i < t.size() && (t[i] == '[' || t[i] == '{')) depth++;
        if (i < t.size() && (t[i] == ']' || t[i] == '}')) depth--;
        if (i == t.size() || (t[i] == ',' && depth == 0)) {
            std::string kv = t.substr(a, i - a);
            size_t e = kv.find('=');
            if (e != std::string::npos) m[trim(kv.substr(0, e))] = trim(kv.substr(e + 1));
            a = i + 1;
        }
    }
    return m;
}
static bool pair_of(const std::string& v, float* xy) { return sscanf(trim(v).c_str(), "[%f , %f]", xy, xy + 1) == 2 || sscanf(trim(v).c_str(), "[%f,%f]", xy, xy + 1) == 2; }
// "centre", "seat:N", "pillbox:N" as a place: G_CENTRE, N, 10 + N.
static int place_of(const std::string& v) {
    std::string s = unq(v);
    if (s.rfind("seat:", 0) == 0) return atoi(s.c_str() + 5);
    if (s.rfind("pillbox:", 0) == 0) return 10 + atoi(s.c_str() + 8);
    return G_CENTRE;
}
static int style_of(const std::string& v) {
    std::string s = unq(v);
    if (s == "net") return G_NET;
    if (s == "idle") return G_IDLE;
    const char* names[] = {"script", "raider", "castle", "expander", "timid", "lurker"};
    for (int i = 0; i < 6; i++)
        if (s == names[i]) return i < STYLES ? i : -99;
    return -99;
}

struct GFile { std::string name, path, reaction; float floor; Scen sc; };

static bool gauntlet_file(const std::string& path, GFile& g, std::string* why) {
    std::vector<TomlT> t = toml_read(path);
    if (t.empty()) { *why = "unreadable"; return false; }
    Scen sc;
    memset(&sc, 0, sizeof sc);
    for (int i = 0; i < 2; i++) { sc.rel[i] = G_CENTRE; sc.head_to[i] = -1; sc.armor[i] = 1.0f; sc.brain[i] = G_IDLE; sc.head_any[i] = 1; }
    sc.hits_min = sc.hits_on_max = sc.alive = sc.pb_dmg_max = sc.standing_max = sc.outscores = sc.win = -1;
    sc.armor_share = -1.0f;
    sc.hold_pb = -1;
    auto& top = t[0].kv;
    g.name = unq(top["name"]);
    g.reaction = top.count("reaction") ? unq(top["reaction"]) : "fire";
    g.floor = top.count("floor") ? (float)atof(top["floor"].c_str()) : 0.0f;
    sc.seconds = top.count("seconds") ? (float)atof(top["seconds"].c_str()) : 10.0f;
    sc.reaction = g.reaction == "move" ? 1 : 0;
    if (top.count("needs") && unq(top["needs"]) == "forest") {
#ifndef FOREST
        *why = "it needs the trees: a FOREST=1 build";
        return false;
#endif
    }
    for (size_t k = 1; k < t.size(); k++) {
        auto& kv = t[k].kv;
        const std::string& kind = t[k].kind;
        if (kind == "terrain") {
            // Grass is the open field crucible's arena already is; a wall is a box of the maze.
            std::string tk = unq(kv["kind"]);
            if (tk == "grass" || tk == "road" || tk.empty()) continue;
            if (tk == "forest") {
                // treadcore's rectangle of forest, here the grove of the same area at its middle.
#ifndef FOREST
                *why = "forest terrain: a FOREST=1 build";
                return false;
#endif
                float xs[2], ys[2];
                if (!pair_of(kv["x"], xs) || !pair_of(kv["y"], ys) || sc.ngr >= G_GROVES) { *why = "a forest without x and y, or too many"; return false; }
                int n = sc.ngr++;
                float w_ = fabsf(xs[1] - xs[0]) + 1.0f, h_ = fabsf(ys[1] - ys[0]) + 1.0f;
                sc.gr_near[n] = kv.count("near") ? place_of(kv["near"]) : 0;
                sc.gr_off[n][0] = 0.5f * (xs[0] + xs[1]); sc.gr_off[n][1] = 0.5f * (ys[0] + ys[1]);
                sc.gr_toward[n] = -1; sc.gr_away[n] = 0.0f;
                sc.gr_r[n] = sqrtf(w_ * h_ / 3.14159265f);
                continue;
            }
            if (tk != "wall") { *why = "terrain of kind " + tk + ": grass, road, forest and wall here"; return false; }
#ifndef FOREST
            *why = "walls: a FOREST=1 build";
            return false;
#endif
            if (sc.nwall >= 12) { *why = "more than 12 walls"; return false; }
            float xs[2], ys[2];
            if (!pair_of(kv["x"], xs) || !pair_of(kv["y"], ys)) { *why = "a wall without x = [a, b] and y = [c, d]"; return false; }
            int n = sc.nwall++;
            sc.w_near[n] = kv.count("near") ? place_of(kv["near"]) : G_CENTRE;
            sc.w_box[n][0] = std::min(xs[0], xs[1]); sc.w_box[n][1] = std::max(xs[0], xs[1]);
            sc.w_box[n][2] = std::min(ys[0], ys[1]); sc.w_box[n][3] = std::max(ys[0], ys[1]);
            continue;
        }
        if (kind == "tank") {
            int s = atoi(kv["seat"].c_str());
            if (s < 0 || s > 1) { *why = "a third seat: crucible's duels are two tanks"; return false; }
            if (kv.count("at")) sc.rel[s] = place_of(kv["at"]) >= 10 ? G_CENTRE : place_of(kv["at"]);
            if (kv.count("offset") && !pair_of(kv["offset"], sc.off[s])) { *why = "an offset not [x, y]"; return false; }
            sc.around[s] = kv.count("around") && unq(kv["around"]) == "true";
            if (kv.count("heading")) {
                std::string h = unq(kv["heading"]);
                if (h == "any") sc.head_any[s] = 1;
                else if (h.rfind("seat:", 0) == 0) { sc.head_any[s] = 0; sc.head_to[s] = atoi(h.c_str() + 5); }
                // treadcore's heading is degrees, 0 east and 90 south: so here, a file read by both.
                else { sc.head_any[s] = 0; sc.head[s] = (float)atof(h.c_str()) * 3.14159265f / 180.0f; }
            }
            if (kv.count("toward")) { int to = place_of(kv["toward"]); if (to >= 0 && to < 10) { sc.head_any[s] = 0; sc.head_to[s] = to; } }
            if (kv.count("armor")) sc.armor[s] = (float)atof(kv["armor"].c_str());
            if (kv.count("reload")) sc.reload[s] = (float)atof(kv["reload"].c_str());
        } else if (kind == "pillbox" || kind == "grove") {
            bool pb = kind == "pillbox";
            int n = pb ? sc.npb : sc.ngr;
            if (n >= (pb ? std::min(G_PB, PILLBOXES) : G_GROVES)) { *why = "more " + kind + "es than the arena holds"; return false; }
#ifndef FOREST
            if (!pb) { *why = "a grove: the trees need a FOREST=1 build"; return false; }
#endif
            float off[2] = {0, 0};
            if (kv.count("offset")) pair_of(kv["offset"], off);
            int near = kv.count("near") ? place_of(kv["near"]) : G_CENTRE, toward = kv.count("toward") ? place_of(kv["toward"]) : -1;
            float away = kv.count("away") ? (float)atof(kv["away"].c_str()) : 0.0f;
            if (toward >= 10) { *why = "toward a pillbox"; return false; }
            if (pb) {
                sc.pb_near[n] = near; sc.pb_off[n][0] = off[0]; sc.pb_off[n][1] = off[1]; sc.pb_toward[n] = toward; sc.pb_away[n] = away;
                // treadcore's owner: 1 is the enemy's, firing only at the learner; none is neutral.
                sc.pb_own[n] = kv.count("owner") ? atoi(kv["owner"].c_str()) : -1;
                sc.npb++;
            }
            else {
                sc.gr_near[n] = near; sc.gr_off[n][0] = off[0]; sc.gr_off[n][1] = off[1]; sc.gr_toward[n] = toward; sc.gr_away[n] = away;
                sc.gr_r[n] = kv.count("radius") ? (float)atof(kv["radius"].c_str()) : 3.0f;
                sc.ngr++;
            }
        } else if (kind == "brain") {
            int s = atoi(kv["seat"].c_str());
            int b = style_of(kv["kind"]);
            if (s < 0 || s > 1 || b == -99 || (b == G_NET && s != 0)) { *why = "a brain this arena has not: " + unq(kv["kind"]); return false; }
            sc.brain[s] = b;
        } else if (kind == "expect") {
            for (auto& e : kv) {
                auto m = inline_table(e.second);
                if (e.first == "hits") sc.hits_min = atoi(m["at_least"].c_str());
                else if (e.first == "hits_on") sc.hits_on_max = atoi(m["max"].c_str());
                else if (e.first == "alive") sc.alive = 1;
                else if (e.first == "pillbox_alive") { if (unq(m["alive"]) == "false") sc.pb_dead |= 1 << atoi(m["pillbox"].c_str()); }
                else if (e.first == "pillbox_damage") sc.pb_dmg_max = atoi(m["max"].c_str());
                else if (e.first == "pillboxes_standing") sc.standing_max = atoi(m["max"].c_str());
                else if (e.first == "armor_at_least") sc.armor_share = (float)atof(m["share"].c_str());
                else if (e.first == "outscores") sc.outscores = 1;
                else if (e.first == "win") sc.win = 1;
                else if (e.first == "pillbox_captured") sc.captured |= 1 << atoi(m["pillbox"].c_str());
                else if (e.first == "pillbox_owned_by") {
                    // treadcore's words for the takeover: the pillbox became seat 0's.
                    if (atoi(m["seat"].c_str()) != 0) { *why = "pillbox_owned_by a seat other than the learner's"; return false; }
                    sc.captured |= 1 << atoi(m["pillbox"].c_str());
                }
                else if (e.first == "hold") { sc.hold_pb = atoi(m["pillbox"].c_str()); sc.hold_s = (float)atof(m["seconds"].c_str()); }
                else { *why = "an expectation this arena does not keep: " + e.first; return false; }
            }
        } else { *why = "a [" + kind + "] this arena does not play"; return false; }
    }
    if (sc.brain[0] != G_NET) { *why = "no net in seat 0"; return false; }
    sc.early = sc.alive < 0 && sc.hold_pb < 0 && sc.hits_on_max < 0 && sc.armor_share < 0.0f && sc.outscores < 0 && sc.pb_dmg_max < 0;
    if (sc.hold_pb >= PILLBOXES || (sc.captured >> PILLBOXES)) { *why = "a pillbox the arena has not"; return false; }
#ifndef FOREST
    bool owned = false;
    for (int q = 0; q < sc.npb; q++) owned = owned || sc.pb_own[q] >= 0;
    if (sc.captured || sc.hold_pb >= 0 || owned) { *why = "the takeover: a FOREST=1 build"; return false; }
#endif
    g.sc = sc;
    g.path = path;
    return true;
}

static std::vector<GFile> gauntlet_files(const std::string& dir) {
    std::vector<std::string> paths;
    if (DIR* d = opendir(dir.c_str())) {
        while (struct dirent* e = readdir(d)) {
            std::string n = e->d_name;
            if (n.size() > 5 && n.substr(n.size() - 5) == ".toml") paths.push_back(dir + "/" + n);
        }
        closedir(d);
    }
    // In the order of their numbers, C1 before C10, as treadcore lists G1 before G10.
    std::sort(paths.begin(), paths.end(), [](const std::string& a, const std::string& b) {
        auto key = [](const std::string& p) {
            std::string n = p.substr(p.find_last_of('/') + 1);
            int num = 0;
            size_t i = 1;
            while (i < n.size() && isdigit((unsigned char)n[i])) num = num * 10 + (n[i++] - '0');
            return std::make_pair(std::string(1, n[0]), num);
        };
        return key(a) < key(b);
    });
    std::vector<GFile> out;
    for (auto& p : paths) {
        GFile g;
        std::string why;
        if (gauntlet_file(p, g, &why)) out.push_back(g);
        else printf("# gauntlet %s: left out, %s\n", p.c_str(), why.c_str());
    }
    return out;
}

// Each net through every file, on the same runs. The tsv is treadcore's gauntlet.tsv, column for column
// (item, brain, passed, runs, share, se, reaction_median, reaction_n), with the file's floor and whether
// the share holds it after. Against the first net, every other one's PAIRED difference over all files:
// runs the first passed and the other failed, and the other way round (McNemar's discordant pairs).
static int run_gauntlet(const std::vector<GFile>& gs, const std::vector<std::pair<std::string, std::vector<float>>>& nets, int runs, uint32_t seed,
                        const std::string& out_path) {
    if (gs.empty()) { printf("# gauntlet: no file to play\n"); return 1; }
    int n = (int)gs.size();
    std::vector<Scen> sc;
    for (auto& g : gs) sc.push_back(g.sc);
    Scen* d_sc;
    float *d_net, *d_rt;
    unsigned int* d_pass;
    unsigned char* d_po;
    CK(cudaMalloc(&d_sc, n * sizeof(Scen)));
    CK(cudaMemcpy(d_sc, sc.data(), n * sizeof(Scen), cudaMemcpyHostToDevice));
    CK(cudaMalloc(&d_net, NW * sizeof(float)));
    CK(cudaMalloc(&d_rt, (size_t)n * runs * sizeof(float)));
    CK(cudaMalloc(&d_pass, n * sizeof(unsigned int)));
    CK(cudaMalloc(&d_po, (size_t)n * runs));
    FILE* f = out_path.empty() ? stdout : fopen(out_path.c_str(), "w");
    if (!f) { perror(out_path.c_str()); return 1; }
    fprintf(f, "item\tbrain\tpassed\truns\tshare\tse\treaction_median\treaction_n\tfloor\tholds\n");
    std::vector<std::vector<unsigned char>> po(nets.size(), std::vector<unsigned char>((size_t)n * runs));
    for (size_t k = 0; k < nets.size(); k++) {
        CK(cudaMemcpy(d_net, nets[k].second.data(), NW * sizeof(float), cudaMemcpyHostToDevice));
        CK(cudaMemset(d_pass, 0, n * sizeof(unsigned int)));
        gauntlet_play<<<n, std::min(runs, 256), NW * sizeof(float)>>>(d_net, d_sc, runs, seed, d_pass, d_rt, d_po);
        CK(cudaGetLastError());
        CK(cudaDeviceSynchronize());
        std::vector<unsigned int> pass(n);
        std::vector<float> rt((size_t)n * runs);
        CK(cudaMemcpy(pass.data(), d_pass, n * sizeof(unsigned int), cudaMemcpyDeviceToHost));
        CK(cudaMemcpy(rt.data(), d_rt, rt.size() * sizeof(float), cudaMemcpyDeviceToHost));
        CK(cudaMemcpy(po[k].data(), d_po, po[k].size(), cudaMemcpyDeviceToHost));
        int held = 0;
        double total = 0;
        for (int i = 0; i < n; i++) {
            std::vector<float> rs;
            for (int r = 0; r < runs; r++)
                if (rt[(size_t)i * runs + r] >= 0.0f) rs.push_back(rt[(size_t)i * runs + r]);
            std::sort(rs.begin(), rs.end());
            double p = pass[i] / (double)runs, se = sqrt(p * (1 - p) / runs);
            bool holds = p >= gs[i].floor - 1e-9;
            held += holds;
            total += p;
            fprintf(f, "%s\t%s\t%u\t%d\t%.4f\t%.4f\t%s\t%zu\t%.2f\t%s\n", gs[i].name.c_str(), nets[k].first.c_str(), pass[i], runs, p, se,
                    rs.empty() ? "-" : (std::to_string(rs[rs.size() / 2]).substr(0, 5)).c_str(), rs.size(), gs[i].floor, holds ? "yes" : "no");
        }
        printf("# gauntlet %s: %d of %d files at their floors, mean share %.3f\n", nets[k].first.c_str(), held, n, total / n);
        if (k > 0) {
            long b = 0, c = 0, N = (long)n * runs;
            for (size_t r = 0; r < po[0].size(); r++) { b += po[0][r] && !po[k][r]; c += !po[0][r] && po[k][r]; }
            double diff = (double)(b - c) / N, se = sqrt(std::max(0.0, (double)(b + c) - (double)(b - c) * (b - c) / N)) / N;
            printf("# gauntlet paired: %s over %s %+.4f +/- %.4f (%ld runs: %ld only %s, %ld only %s)\n", nets[0].first.c_str(), nets[k].first.c_str(),
                   diff, se, N, b, nets[0].first.c_str(), c, nets[k].first.c_str());
            fprintf(f, "PAIRED\t%s|%s\t%ld\t%ld\t%.4f\t%.4f\t-\t%ld\t-\t-\n", nets[0].first.c_str(), nets[k].first.c_str(), b, c, diff, se, N);
            // Each file's own discordant pairs, so a reader can weigh the files (by how hard they are).
            for (int i = 0; i < n; i++) {
                long bi = 0, ci = 0;
                for (int r = 0; r < runs; r++) {
                    size_t x = (size_t)i * runs + r;
                    bi += po[0][x] && !po[k][x]; ci += !po[0][x] && po[k][x];
                }
                fprintf(f, "PAIRED-ITEM\t%s|%s\t%ld\t%ld\t-\t-\t-\t%d\t-\t%s\n", nets[0].first.c_str(), nets[k].first.c_str(), bi, ci, runs, gs[i].name.c_str());
            }
        }
    }
    if (f != stdout) fclose(f);
    cudaFree(d_sc); cudaFree(d_net); cudaFree(d_rt); cudaFree(d_pass); cudaFree(d_po);
    return 0;
}

static std::string base_of(const std::string& path) {
    size_t s = path.find_last_of('/');
    std::string b = s == std::string::npos ? path : path.substr(s + 1);
    return b.size() > 4 && b.substr(b.size() - 4) == ".bin" ? b.substr(0, b.size() - 4) : b;
}
#endif
#endif

static double now() { return std::chrono::duration<double>(std::chrono::steady_clock::now().time_since_epoch()).count(); }

int main(int argc, char** argv) {
    Opt o;
    for (int i = 1; i < argc; i++) {
        std::string a = argv[i];
        auto val = [&]() -> const char* { if (i + 1 >= argc) usage(); return argv[++i]; };
        if (a == "--pop") o.pop = atoi(val());
        else if (a == "--eps") o.eps = atoi(val());
        else if (a == "--steps") o.steps = atoi(val());
        else if (a == "--gens") o.gens = atoi(val());
        else if (a == "--elite") o.elite = atoi(val());
        else if (a == "--every") o.every = atoi(val());
        else if (a == "--cap") o.cap = atoi(val());
        else if (a == "--sigma") o.sigma = atof(val());
        else if (a == "--spar") o.spar = atof(val());
        else if (a == "--seed") o.seed = (uint32_t)atol(val());
        else if (a == "--yard") o.yard = atoi(val());
        else if (a == "--out") o.out = val();
        else if (a == "--save") o.save = val();
        else if (a == "--load") o.load = val();
        else if (a == "--export") o.exp = val();
        else if (a == "--import") o.imp = val();
        else if (a == "--ladder") o.ladder = val();
        else if (a == "--lad") o.lad = atof(val());
        else if (a == "--draw") o.draw = atof(val());
        else if (a == "--exam") o.exam = val();
        else if (a == "--exam-eps") o.exam_eps = atoi(val());
        else if (a == "--bench") o.bench = true;
        else if (a == "--trace") o.trace = val();
        else if (a == "--champions") o.champs = val();
        else if (a == "--opponents") o.opps = val();
        else if (a == "--trace-duels") o.trace_duels = atoi(val());
        else if (a == "--trace-seed") o.trace_seed = (uint32_t)atol(val());
        else if (a == "--trace-spread") o.trace_spread = (float)atof(val());
        else if (a == "--gauntlet") o.gauntlet = val();
        else if (a == "--gauntlet-out") o.gauntlet_out = val();
        else if (a == "--gauntlet-ref") o.gauntlet_ref = val();
        else if (a == "--gauntlet-runs") o.gauntlet_runs = atoi(val());
        else if (a == "--gauntlet-seed") o.gauntlet_seed = (uint32_t)atol(val());
        else if (a == "--tasks") o.tasks = val();
        else if (a == "--task-p") o.task_p = (float)atof(val());
        else usage();
    }

    cudaDeviceProp dp;
    CK(cudaGetDeviceProperties(&dp, 0));
    cudaFuncAttributes fa;
    CK(cudaFuncGetAttributes(&fa, play));
    size_t shm = 2 * NW * sizeof(float);
    int per_sm = 0;
    CK(cudaOccupancyMaxActiveBlocksPerMultiprocessor(&per_sm, play, o.eps, shm));
    size_t free_b, total_b;
    CK(cudaMemGetInfo(&free_b, &total_b));
    // The clock as an attribute, not cudaDeviceProp::clockRate: CUDA 13
    // removed the field, and the attribute is in 12 and 13 alike, so one
    // source builds on the Pascal cards (12.9) and the Ada one (13).
    int khz = 0;
    CK(cudaDeviceGetAttribute(&khz, cudaDevAttrClockRate, 0));
    printf("# device %s, sm_%d%d, %d SMs, %.0f MHz, %.1f GB (%.1f free)\n", dp.name, dp.major, dp.minor, dp.multiProcessorCount,
           khz / 1000.0, total_b / 1e9, free_b / 1e9);
    printf("# net %s (%s): %d weights (%.1f KB); play: %d registers/thread, %zu B shared/block, %d blocks of %d per SM (occupancy %.0f%%)\n",
           shape(HERE).c_str(), KIND, NW, NW * 4 / 1024.0, fa.numRegs, shm, per_sm, o.eps, 100.0 * per_sm * o.eps / dp.maxThreadsPerMultiProcessor);
#ifdef WORLD
    printf("# world: treadcore's rules, %d pillboxes, %d steps a duel (%.0f s)\n", PILLBOXES, o.steps, o.steps * DT);
    printf("# rules: speed %g accel %g brake %g turn %g; armour %g, %g shells; gun %g tiles at %g/s for %g every %g s, hit radius %g; "
           "vision %g; pillboxes %g tiles for %g every %g s, health %g\n",
           VMAX, ACCEL, BRAKE, TURN, HP, SHELLS, GUN_RANGE, SSPEED, DAMAGE, RELOAD, HIT_R, VISION, PB_RANGE, PB_DAMAGE, PB_RELOAD, PB_HEALTH);
#endif
    if (!o.trace.empty()) return run_trace(o);
#ifdef WORLD
    if (!o.gauntlet.empty() && !o.champs.empty()) {
        std::vector<std::pair<std::string, std::vector<float>>> nets;
        for (auto& path : split(o.champs)) {
            std::vector<float> w; std::string note;
            if (!read_champion(path, w, &note)) { fprintf(stderr, "%s: not a champion of this net\n", path.c_str()); return 1; }
            nets.push_back({base_of(path), w});
        }
        return run_gauntlet(gauntlet_files(o.gauntlet), nets, o.gauntlet_runs, o.gauntlet_seed, o.gauntlet_out);
    }
#endif

    size_t pop_b = (size_t)o.pop * NW * sizeof(float), hall_b = (size_t)o.cap * NW * sizeof(float);
    printf("# memory: population %.1f MB x2, hall of fame %.1f MB (%d champions)\n", pop_b / 1e6, hall_b / 1e6, o.cap);
    float *pop, *next, *hall, *fit;
    int *rank, *parent;
    Totals* tot;
    CK(cudaMalloc(&pop, pop_b));
    CK(cudaMalloc(&next, pop_b));
    CK(cudaMalloc(&hall, hall_b));
    CK(cudaMalloc(&fit, o.pop * sizeof(float)));
    CK(cudaMalloc(&rank, o.pop * sizeof(int)));
    CK(cudaMalloc(&parent, o.pop * sizeof(int)));
    CK(cudaMallocManaged(&tot, sizeof(Totals)));

    int gen0 = 0, hall_n = 0, hall_next = 0;
    if (!o.load.empty()) {
        FILE* f = fopen(o.load.c_str(), "rb");
        if (!f) { perror(o.load.c_str()); return 1; }
        // The header: a mark, the net's size, --pop, --cap, the generation,
        // how many the hall holds and where its next one goes (imports move
        // that, so it is kept rather than worked out from the generation).
        // v3 (0x63727533) adds the shape, nin, h, nout, and whether it is a
        // world net; v2 (0x63727532) has only the size, and the shape is
        // worked out from it.
        int hdr[11] = {0};
        bool ok = fread(hdr, sizeof(int), 7, f) == 7;
        Dims s = {0, 0, 0};
        if (ok && hdr[0] == 0x63727533) {
            ok = fread(hdr + 7, sizeof(int), 4, f) == 4;
            s = Dims{hdr[7], hdr[8], hdr[9]};
            ok = ok && nw_of(s) == hdr[1];
        } else if (ok && hdr[0] == 0x63727532) {
            ok = dims_of(hdr[1], &s);
        } else {
            ok = false;
        }
        if (!ok || hdr[2] != o.pop || hdr[3] != o.cap) {
            fprintf(stderr, "%s: not a crucible state for this --pop and --cap\n", o.load.c_str());
            return 1;
        }
        gen0 = hdr[4]; hall_n = hdr[5]; hall_next = hdr[6];
        // Read one block of nets (the population, then the hall) in the
        // file's shape, widened into this build's when it is smaller.
        const char* why = nullptr;
        auto block = [&](float* dev, size_t n) -> bool {
            std::vector<float> h(n * NW);
            if (same(s, HERE)) {
                if (fread(h.data(), 4, h.size(), f) != h.size()) return false;
            } else {
                std::vector<float> one(hdr[1]);
                for (size_t i = 0; i < n; i++) {
                    if (fread(one.data(), 4, one.size(), f) != one.size()) return false;
                    if ((why = widen(one.data(), s, h.data() + i * NW))) return false;
                }
            }
            CK(cudaMemcpy(dev, h.data(), h.size() * sizeof(float), cudaMemcpyHostToDevice));
            return true;
        };
        if (!block(pop, o.pop) || !block(hall, o.cap)) {
            fprintf(stderr, "%s: %s\n", o.load.c_str(), why ? why : "short: not a whole state");
            return 1;
        }
        fclose(f);
        if (!same(s, HERE)) printf("# widened from %s to %s: every new weight zero, the old net's answers unchanged\n", shape(s).c_str(), shape(HERE).c_str());
        printf("# resumed %s at generation %d, %d in the hall of fame\n", o.load.c_str(), gen0, hall_n);
    } else {
        seed_pop<<<(unsigned)(((size_t)o.pop * NW + 255) / 256), 256>>>(pop, (size_t)o.pop * NW, o.seed);
        // The hall starts with one: a random net, the first thing to beat.
        CK(cudaMemcpy(hall, pop, NW * sizeof(float), cudaMemcpyDeviceToDevice));
        hall_n = 1; hall_next = 1;
    }

    // Islands (the fleet's cards, each its own population): the champions
    // other islands sent join this one's hall of fame, so each learns to
    // beat the others' best as well as its own past.
    if (!o.imp.empty()) {
        size_t a = 0;
        int n = 0;
        while (a <= o.imp.size()) {
            size_t b = o.imp.find(',', a);
            if (b == std::string::npos) b = o.imp.size();
            std::string path = o.imp.substr(a, b - a);
            a = b + 1;
            if (path.empty()) continue;
            std::vector<float> w;
            std::string note;
            if (!read_champion(path, w, &note)) {
                fprintf(stderr, "# import %s: not a champion of this net, skipped\n", path.c_str());
                continue;
            }
            if (!note.empty()) printf("# import %s: %s\n", path.c_str(), note.c_str());
            CK(cudaMemcpy(hall + (size_t)hall_next * NW, w.data(), NW * sizeof(float), cudaMemcpyHostToDevice));
            hall_next = (hall_next + 1) % o.cap;
            hall_n = std::min(hall_n + 1, o.cap);
            n++;
        }
        printf("# imported %d champion(s) into the hall of fame (%d there)\n", n, hall_n);
    }

    // The ladder: milestones, oldest first, kept on the card beside the hall.
    std::vector<std::string> rungs;
    for (size_t a = 0; !o.ladder.empty() && a <= o.ladder.size();) {
        size_t b = o.ladder.find(',', a);
        if (b == std::string::npos) b = o.ladder.size();
        if (b > a) rungs.push_back(o.ladder.substr(a, b - a));
        a = b + 1;
    }
    std::vector<float> lh;
    std::vector<std::string> names;
    for (auto& path : rungs) {
        std::vector<float> w;
        std::string note;
        if (!read_champion(path, w, &note)) {
            fprintf(stderr, "# ladder %s: not a champion of this net, skipped\n", path.c_str());
            continue;
        }
        if (!note.empty()) printf("# ladder %s: %s\n", path.c_str(), note.c_str());
        lh.insert(lh.end(), w.begin(), w.end());
        size_t s = path.find_last_of('/');
        names.push_back(s == std::string::npos ? path : path.substr(s + 1));
    }
    int ladder_n = (int)names.size();
    float* ladder = nullptr;
    CK(cudaMalloc(&ladder, std::max<size_t>(1, lh.size()) * sizeof(float)));
    if (ladder_n) CK(cudaMemcpy(ladder, lh.data(), lh.size() * sizeof(float), cudaMemcpyHostToDevice));
    if (ladder_n) printf("# ladder: %d milestone(s), opponents %.0f%% of the time\n", ladder_n, 100.0 * o.lad);
    const float lad = ladder_n ? o.lad : 0.0f;

    // One net against one opponent (the yardstick, mode 1, or a milestone,
    // mode 2), n duels: wins, draws and losses as fractions.
    float* yf;
    CK(cudaMalloc(&yf, sizeof(float)));
    auto match = [&](const float* net, int mode, int rung, int n, uint32_t s, int g, float* wdl) {
        int reps = std::max(1, n / o.eps);
        CK(cudaMemset(yf, 0, sizeof(float)));
        memset(tot, 0, sizeof(Totals));
        play<<<reps, o.eps, shm>>>(net, hall, hall_n, o.cap, hall_next, ladder, ladder_n, 0.0f, rung, 0.0f, reps, mode, 0.0f, o.steps, s, g, yf, tot, nullptr, 0, 0.0f);
        CK(cudaDeviceSynchronize());
        CK(cudaGetLastError());
        float m = (float)(tot->wins + tot->losses + tot->draws);
        wdl[0] = tot->wins / m; wdl[1] = tot->draws / m; wdl[2] = tot->losses / m;
    };

    FILE* out = o.out.empty() ? nullptr : fopen(o.out.c_str(), "a");
    if (out) fseek(out, 0, SEEK_END);
    // A header on a new file, whichever generation it starts at (an
    // island's segment starts where the last one stopped).
    // The ladder's two columns come after the old ten, so a reader of those
    // is not disturbed. A score is wins less losses, -1 to 1: wins plus half
    // the draws would give a net that never fights 0.5, since a duel it
    // hides from is a draw (a 20-generation net drew 87% and 100% of its
    // duels with two islands' champions).
    // Under WORLD four more, after the twelve, so no reader of those is moved:
    // the learner's pillbox damage taken a duel (in hits of DAMAGE), the
    // pillboxes it destroyed a duel, the shells it fired a duel, and the
    // share of those that struck a tank or a pillbox.
#ifdef WORLD
    const char* more = "\tpill_taken\tpill_killed\tshots\thit_rate";
    const char* more_c = "\tpill taken/killed\tshots\thit";
#else
    const char* more = "";
    const char* more_c = "";
#endif
    if (out && ftell(out) == 0) fprintf(out, "gen\tbest\tmean\tplay_s\tsteps_per_s\tnet_gflops\thall\tyard_win\tyard_draw\tyard_loss\tladder_score\tladder_top%s\n", more);
    printf("gen\tbest\tmean\tplay_s\tMsteps/s\tnet GFLOP/s\thall\tyardstick w/d/l\tladder all/top%s\n", more_c);

    Scen* d_tasks = nullptr;
    int n_tasks = 0;
#ifdef WORLD
    if (!o.tasks.empty()) {
        std::vector<GFile> tf = gauntlet_files(o.tasks);
        std::vector<Scen> ts;
        for (auto& g : tf) ts.push_back(g.sc);
        n_tasks = (int)ts.size();
        if (n_tasks) {
            CK(cudaMalloc(&d_tasks, n_tasks * sizeof(Scen)));
            CK(cudaMemcpy(d_tasks, ts.data(), n_tasks * sizeof(Scen), cudaMemcpyHostToDevice));
        }
        printf("# tasks: %d of the gauntlet's files, a training duel one of them with chance %.2f\n", n_tasks, o.task_p);
    }
#endif
    double flops_per_step = 2.0 * (NIN * H + H * H + H * NOUT); // one net, multiply-adds as two
    cudaEvent_t e0, e1;
    CK(cudaEventCreate(&e0));
    CK(cudaEventCreate(&e1));
    thrust::device_ptr<float> dfit(fit);
    thrust::device_ptr<int> drank(rank);
    double t_start = now();

    for (int g = gen0; g < gen0 + o.gens; g++) {
        CK(cudaMemset(fit, 0, o.pop * sizeof(float)));
        memset(tot, 0, sizeof(Totals));
        CK(cudaEventRecord(e0));
        play<<<o.pop, o.eps, shm>>>(pop, hall, hall_n, o.cap, hall_next, ladder, ladder_n, lad, 0, o.draw, 1, 0, o.spar, o.steps, o.seed, g, fit, tot, d_tasks, n_tasks, o.task_p);
        CK(cudaEventRecord(e1));
        CK(cudaEventSynchronize(e1));
        CK(cudaGetLastError());
        float ms;
        CK(cudaEventElapsedTime(&ms, e0, e1));
        double sps = tot->steps / (ms / 1000.0);
        // Every tank-step is a net's decision when both are nets.
        double gfl = sps * flops_per_step / 1e9;
#ifdef WORLD
        // Read now: the yardstick's matches below reset the totals.
        double duels = (double)tot->wins + tot->losses + tot->draws;
        if (duels < 1) duels = 1;
        double pill_taken = tot->pill_struck * (double)(PB_DAMAGE / DAMAGE) / duels, pill_killed = tot->pill_killed / duels;
        double shots = tot->shots / duels, hit_rate = tot->shots ? (double)tot->hits / tot->shots : 0.0;
#endif
        if (n_tasks && o.yard > 0 && (g + 1) % o.yard == 0)
            printf("# tasks gen %d: %u run, %u passed (%.1f%%)\n", g, tot->tasks, tot->tasks_passed, 100.0 * tot->tasks_passed / std::max(1u, tot->tasks));
        if (o.bench) {
            printf("%d\t-\t-\t%.3f\t%.1f\t%.0f\t-\t-\n", g, ms / 1000.0, sps / 1e6, gfl);
            continue;
        }

        thrust::sequence(drank, drank + o.pop);
        thrust::sort_by_key(dfit, dfit + o.pop, drank, thrust::greater<float>());
        float best, mean = thrust::reduce(dfit, dfit + o.pop) / o.pop;
        CK(cudaMemcpy(&best, fit, sizeof(float), cudaMemcpyDeviceToHost));
        int champ;
        CK(cudaMemcpy(&champ, rank, sizeof(int), cudaMemcpyDeviceToHost));

#ifdef HYBRID
        if (o.yard > 0 && (g + 1) % o.yard == 0)
            printf("#hybrid gen %d: the net overrode the rules on %.1f%% of steps\n", g, 100.0 * tot->opened / (tot->decided ? tot->decided : 1));
#endif
        char yard[64] = "-", ladc[32] = "-";
        float yw = -1, yd = -1, yl = -1, ls = -9, lt = -9; // -9: not measured (a score can be -1)
        if (o.yard > 0 && (g + 1) % o.yard == 0) {
            float wdl[3];
            match(pop + (size_t)champ * NW, 1, 0, o.yard_eps, o.seed ^ 0xa5a5u, g, wdl);
            yw = wdl[0]; yd = wdl[1]; yl = wdl[2];
            snprintf(yard, sizeof yard, "%.2f/%.2f/%.2f", yw, yd, yl);
            // The ladder: the yardstick's duels shared among the milestones.
            if (ladder_n) {
                ls = 0;
                for (int k = 0; k < ladder_n; k++) {
                    match(pop + (size_t)champ * NW, 2, k, std::max(o.eps, o.yard_eps / ladder_n), o.seed ^ 0x5a5au, g, wdl);
                    float sc = wdl[0] - wdl[2];
                    ls += sc / ladder_n;
                    if (k == ladder_n - 1) lt = sc;
                }
                snprintf(ladc, sizeof ladc, "%.2f/%.2f", ls, lt);
            }
        }
        if ((g + 1) % o.every == 0) {
            CK(cudaMemcpy(hall + (size_t)hall_next * NW, pop + (size_t)champ * NW, NW * sizeof(float), cudaMemcpyDeviceToDevice));
            hall_next = (hall_next + 1) % o.cap;
            hall_n = std::min(hall_n + 1, o.cap);
        }
        choose<<<(o.pop + 255) / 256, 256>>>(rank, o.pop, o.elite, o.seed, g, parent);
        breed<<<(unsigned)(((size_t)o.pop * NW + 255) / 256), 256>>>(pop, parent, o.pop, o.elite, o.sigma, o.seed, g, next);
        std::swap(pop, next);
        CK(cudaGetLastError());

#ifdef WORLD
        printf("%d\t%.3f\t%.3f\t%.3f\t%.1f\t%.0f\t%d\t%s\t%s\t%.3f/%.3f\t%.2f\t%.3f\n", g, best, mean, ms / 1000.0, sps / 1e6, gfl, hall_n, yard, ladc,
               pill_taken, pill_killed, shots, hit_rate);
#else
        printf("%d\t%.3f\t%.3f\t%.3f\t%.1f\t%.0f\t%d\t%s\t%s\n", g, best, mean, ms / 1000.0, sps / 1e6, gfl, hall_n, yard, ladc);
#endif
        fflush(stdout);
        if (out) {
#ifdef WORLD
            fprintf(out, "%d\t%.4f\t%.4f\t%.4f\t%.0f\t%.1f\t%d\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.3f\t%.4f\n", g, best, mean, ms / 1000.0, sps, gfl, hall_n, yw, yd, yl, ls, lt,
                    pill_taken, pill_killed, shots, hit_rate);
#else
            fprintf(out, "%d\t%.4f\t%.4f\t%.4f\t%.0f\t%.1f\t%d\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\n", g, best, mean, ms / 1000.0, sps, gfl, hall_n, yw, yd, yl, ls, lt);
#endif
            fflush(out);
        }
    }
    CK(cudaDeviceSynchronize());
    printf("# %d generations in %.1f s\n", o.gens, now() - t_start);

    if (!o.exp.empty()) {
        // The elite keep their places, best first: after the last breed
        // the champion is individual 0.
        std::vector<float> w(NW);
        CK(cudaMemcpy(w.data(), pop, NW * sizeof(float), cudaMemcpyDeviceToHost));
        FILE* f = fopen(o.exp.c_str(), "wb");
        fwrite(w.data(), 4, NW, f);
        fclose(f);
        printf("# exported the champion to %s (%d weights)\n", o.exp.c_str(), NW);
    }
    if (!o.exam.empty()) {
        // The exam: the final champion (individual 0) against the yardstick
        // and each milestone, --exam-eps duels each, on seeds of its own.
        // A line a match: what, wins, draws, losses, duels.
        FILE* f = fopen(o.exam.c_str(), "w");
        fprintf(f, "opponent\twin\tdraw\tloss\tduels\n");
        float wdl[3];
        int n = std::max(1, o.exam_eps / o.eps) * o.eps;
        match(pop, 1, 0, o.exam_eps, o.seed ^ 0xe8a3u, gen0 + o.gens, wdl);
        fprintf(f, "yardstick\t%.4f\t%.4f\t%.4f\t%d\n", wdl[0], wdl[1], wdl[2], n);
        // The newest milestone against the yardstick on the same seeds, so a champion's step over it is a
        // difference of two reads of the same duels (yardstick@NAME; a reader of rungs leaves it out).
        if (ladder_n) {
            float nw[3];
            match(ladder + (size_t)(ladder_n - 1) * NW, 1, 0, o.exam_eps, o.seed ^ 0xe8a3u, gen0 + o.gens, nw);
            fprintf(f, "yardstick@%s\t%.4f\t%.4f\t%.4f\t%d\n", names[ladder_n - 1].c_str(), nw[0], nw[1], nw[2], n);
        }
        for (int k = 0; k < ladder_n; k++) {
            match(pop, 2, k, o.exam_eps, o.seed ^ 0xe8a3u, gen0 + o.gens, wdl);
            fprintf(f, "%s\t%.4f\t%.4f\t%.4f\t%d\n", names[k].c_str(), wdl[0], wdl[1], wdl[2], n);
            printf("# exam: %s %.3f/%.3f/%.3f\n", names[k].c_str(), wdl[0], wdl[1], wdl[2]);
        }
        fclose(f);
        printf("# exam written to %s (%d milestone(s))\n", o.exam.c_str(), ladder_n);
    }
#ifdef WORLD
    if (!o.gauntlet.empty()) {
        // The final champion through the gauntlet, and the reference (the newest milestone) on the same runs.
        std::vector<std::pair<std::string, std::vector<float>>> nets(1);
        nets[0].first = "champion";
        nets[0].second.resize(NW);
        CK(cudaMemcpy(nets[0].second.data(), pop, NW * sizeof(float), cudaMemcpyDeviceToHost));
        for (auto& path : split(o.gauntlet_ref)) {
            std::vector<float> w; std::string note;
            if (read_champion(path, w, &note)) nets.push_back({base_of(path), w});
            else printf("# gauntlet reference %s: not a champion of this net, left out\n", path.c_str());
        }
        run_gauntlet(gauntlet_files(o.gauntlet), nets, o.gauntlet_runs, o.gauntlet_seed, o.gauntlet_out);
    }
#endif
    if (!o.save.empty()) {
        FILE* f = fopen(o.save.c_str(), "wb");
        // A world net writes v3, with its shape. The others keep writing v2:
        // the fleet's islands are sent this source with every segment, and a
        // v2 state is one the program before this one could still load, so a
        // step back loses no island's generations. Their shape is still found
        // from the size alone.
#ifdef WORLD
        int hdr[11] = {0x63727533, NW, o.pop, o.cap, gen0 + o.gens, hall_n, hall_next, NIN, H, NOUT, 1};
        fwrite(hdr, sizeof(int), 11, f);
#else
        int hdr[7] = {0x63727532, NW, o.pop, o.cap, gen0 + o.gens, hall_n, hall_next};
        fwrite(hdr, sizeof(int), 7, f);
#endif
        std::vector<float> h((size_t)o.pop * NW);
        CK(cudaMemcpy(h.data(), pop, pop_b, cudaMemcpyDeviceToHost));
        fwrite(h.data(), 4, h.size(), f);
        h.resize((size_t)o.cap * NW);
        CK(cudaMemcpy(h.data(), hall, hall_b, cudaMemcpyDeviceToHost));
        fwrite(h.data(), 4, h.size(), f);
        fclose(f);
        printf("# saved %s: the population and the hall of fame, generation %d\n", o.save.c_str(), gen0 + o.gens);
    }
    if (out) fclose(out);
    return 0;
}
