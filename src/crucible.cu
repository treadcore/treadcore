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
// treadcore-sim's rules.rs, by name.
#define VMAX 4.0f              // TANK_SPEED, tiles/s
#define ACCEL 4.0f             // TANK_ACCEL
#define BRAKE 6.0f             // TANK_BRAKE
#define TURN 3.0f              // TANK_TURN, rad/s
#define TURN_BRAKE_ANGLE 0.6f  // a heading error past this eases off
#define TURN_CRAWL 0.35f       // down to this share of speed for a turn right round
#define TURN_PIVOT 2.0f        // past this it pivots where it stands
#define GUN_RANGE 10.0f        // DIRECT_RANGE
#define SSPEED 40.0f           // SHELL_SPEED_DIRECT
#define SLIFE (GUN_RANGE / SSPEED)
#define RELOAD 4.0f            // RELOAD_TICKS, in seconds
#define HP 100.0f              // TANK_ARMOR
#define DAMAGE 45.0f           // DIRECT_DAMAGE: a hit, the unit fitness counts in
#define HIT_R 0.8f             // DIRECT_HIT_RADIUS
#define SHELLS 24.0f           // TANK_SHELLS
#define VISION 10.0f           // TANK_VISION: own eyes, as far as the gun
#define PILLBOXES 4
#define PB_RANGE 8.0f          // PILLBOX_RANGE
#define PB_DAMAGE 15.0f        // PILLBOX_DAMAGE
#define PB_RELOAD 2.0f         // PILLBOX_RELOAD_TICKS, in seconds
#define PB_HEALTH 60.0f        // PILLBOX_HEALTH: two direct hits
#define PB_HIT (HIT_R + 0.5f)  // a shell strikes a pillbox this near, as treadcore's does
#define PB_R 0.5f              // its body, to drive round
#define STEPS_DEFAULT 1200     // two minutes: with a 4 s reload, 600 steps is fifteen shots
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
};

struct Arena {
    float px[PILLARS], py[PILLARS], pr[PILLARS];
#ifdef WORLD
    // The neutral pillboxes: where, health, and seconds until the next shot.
    float bx[PILLBOXES], by[PILLBOXES], bhp[PILLBOXES], brl[PILLBOXES];
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
    return true;
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
        float te = en.hp > 0.0f ? ray_circle(me.x, me.y, dx, dy, en.x, en.y, TANK_R) : 1e9f;
#ifdef WORLD
        // Own eyes reach VISION: the walls and pillars are the map, known;
        // a tank further off is not seen, in either channel.
        if (te > VISION) te = 1e9f;
        float tp = 1e9f;
#pragma unroll
        for (int k = 0; k < PILLBOXES; k++)
            if (w.bhp[k] > 0.0f) tp = fminf(tp, ray_circle(me.x, me.y, dx, dy, w.bx[k], w.by[k], PB_HIT));
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
        if (w.bhp[k] <= 0.0f) continue;
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
struct Skill { float tol, range; };
#ifdef WORLD
// Inside the gun's 10, as a treadcore bot stands off; sparring partners
// stand anywhere from 4 to 9.
#define YARDSTICK Skill{0.12f, 8.0f}
#define SPAR_NEAR 4.0f
#define SPAR_FAR 9.0f
#define FIRE_REACH GUN_RANGE
#else
#define YARDSTICK Skill{0.12f, 11.0f}
#define SPAR_NEAR 6.0f
#define SPAR_FAR 20.0f
#define FIRE_REACH 35.0f
#endif

__device__ void scripted(const Tank& me, const Tank& en, Skill k, float* out) {
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
}

// What a net-driven tank does. Returns 1 when the net's own say reached the tank.
__device__ int decide(const float* __restrict__ wt, const Tank& me, const Tank& en, const Arena& w, float* in, float* out) {
    sense(me, en, w, in);
#ifdef HYBRID
    float rules[3];
    scripted(me, en, YARDSTICK, rules);
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
    if (en.hp > 0.0f) {
        float te = ray_circle_in(t.sx, t.sy, dx, dy, en.x, en.y, HIT_R);
        if (te < hit) { hit = te; what = 1; }
    }
#pragma unroll
    for (int k = 0; k < PILLBOXES; k++) {
        if (w.bhp[k] <= 0.0f) continue;
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
        bool sa = da <= PB_RANGE && line_clear(w.bx[k], w.by[k], a.x, a.y, w);
        bool sb = db <= PB_RANGE && line_clear(w.bx[k], w.by[k], b.x, b.y, w);
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
            if (ok) break;
        }
        w.bhp[k] = PB_HEALTH; w.brl[k] = 0.0f;
    }
#endif
    place(A, r, 3.0f, 14.0f, w);
    place(B, r, 50.0f, 61.0f, w);
}

// ---- one generation's play ----------------------------------------------------------

struct Totals {
    unsigned long long steps; // tank-steps actually played (both tanks)
    unsigned long long opened, decided; // the learner's steps where the net had its say, of its steps
    unsigned int wins, losses, draws;
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
                     int reps, int opp_mode, float spar, int steps, uint32_t seed, int gen, float* fitness, Totals* tot) {
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
        if (u < spar) skill = Skill{br.in(0.05f, 0.6f), br.in(SPAR_NEAR, SPAR_FAR)};
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

    float in[NIN], actA[NOUT], actB[NOUT];
    float aim = 0;
    unsigned int opened = 0;
#ifdef WORLD
    unsigned int a_hit_tank = 0, a_hit_box = 0, a_killed = 0, a_struck = 0;
#endif
    int t = 0;
    for (; t < steps; t++) {
        if (A.hp <= 0.0f || B.hp <= 0.0f) break;
        opened += decide(wA, A, B, w, in, actA);
        aim += in[29] > 0.0f ? in[29] * (in[28] * in[28] < 0.01f ? 1.0f : 0.2f) : 0.0f;
        if (net_opp) decide(wB, B, A, w, in, actB);
        else scripted(B, A, skill, actB);
        drive(A, actA, w);
        drive(B, actB, w);
#ifdef WORLD
        int fa = fly_world(A, B, w);
        fly_world(B, A, w);
        if (fa == 1) a_hit_tank++;
        if (fa >= 2) a_hit_box++;
        if (fa == 3) a_killed++;
        a_struck += guns(w, A, B);
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
// shots struck each tank.
#define TREC 22
__global__ void __launch_bounds__(256, 1) trace_play(const float* __restrict__ champ, const float* __restrict__ opps, int n_opps, int steps,
                                                     uint32_t seed, float drawv, float* rec, int* outcome, int* lasted, float* fit, int* opp_of,
                                                     float* arena_out) {
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
    else skill = Skill{dice.in(0.05f, 0.6f), dice.in(SPAR_NEAR, SPAR_FAR)};
    opp_of[d] = oi;
    if (d == 0) {
        for (int p = 0; p < PILLARS; p++) { arena_out[p] = w.px[p]; arena_out[PILLARS + p] = w.py[p]; arena_out[2 * PILLARS + p] = w.pr[p]; }
        for (int k = 0; k < PILLBOXES; k++) { arena_out[3 * PILLARS + k] = w.bx[k]; arena_out[3 * PILLARS + PILLBOXES + k] = w.by[k]; }
    }
    float in[NIN], actA[NOUT], actB[NOUT];
    float aim = 0;
    unsigned int hit_tank = 0;
    float* out = rec + (size_t)d * steps * TREC;
    int t = 0;
    for (; t < steps; t++) {
        if (A.hp <= 0.0f || B.hp <= 0.0f) break;
        decide(champ, A, B, w, in, actA);
        aim += in[29] > 0.0f ? in[29] * (in[28] * in[28] < 0.01f ? 1.0f : 0.2f) : 0.0f;
        if (opp) decide(opp, B, A, w, in, actB);
        else scripted(B, A, skill, actB);
        drive(A, actA, w);
        drive(B, actB, w);
        int fa = fly_world(A, B, w);
        int fb = fly_world(B, A, w);
        float bh = B.hp;
        int pa = guns(w, A, B);
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
    bool bench = false;
};

static void usage() {
    fprintf(stderr,
            "crucible [--pop N] [--eps N] [--steps N] [--gens N] [--elite N] [--every K] [--cap N]\n"
            "         [--sigma S] [--spar P] [--seed N] [--yard K] [--out FILE.tsv] [--save FILE] [--load FILE] [--bench]\n"
            "         [--export CHAMP.bin] [--import A.bin,B.bin,...] [--ladder M1.bin,M2.bin,...] [--lad P]\n"
            "         [--exam FILE.tsv] [--exam-eps N] [--draw V]\n"
            "crucible --trace OUT.bin --champions C1.bin,C2.bin,... [--opponents M1.bin,...] [--trace-duels N] [--trace-seed S]\n"
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
// (pillars' x, y, r, pillboxes' x, y); each duel's opponent (-1 a sparring
// partner, else the --opponents index); then for each champion its duels'
// outcomes (1 won, -1 lost, 0 drawn), steps played and fitness, and every
// step of every duel, TREC floats. A duel that ends early stops there: its
// later steps are zero, and `steps played` says where.
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
    size_t recn = (size_t)nd * o.steps * TREC;
    float *d_ch, *d_op = nullptr, *d_rec, *d_fit, *d_ar;
    int *d_out, *d_last, *d_opp;
    CK(cudaMalloc(&d_ch, champs.size() * sizeof(float)));
    CK(cudaMemcpy(d_ch, champs.data(), champs.size() * sizeof(float), cudaMemcpyHostToDevice));
    if (!opps.empty()) {
        CK(cudaMalloc(&d_op, opps.size() * sizeof(float)));
        CK(cudaMemcpy(d_op, opps.data(), opps.size() * sizeof(float), cudaMemcpyHostToDevice));
    }
    CK(cudaMalloc(&d_rec, recn * sizeof(float)));
    CK(cudaMalloc(&d_fit, nd * sizeof(float)));
    CK(cudaMalloc(&d_ar, (3 * PILLARS + 2 * PILLBOXES) * sizeof(float)));
    CK(cudaMalloc(&d_out, nd * sizeof(int)));
    CK(cudaMalloc(&d_last, nd * sizeof(int)));
    CK(cudaMalloc(&d_opp, nd * sizeof(int)));
    FILE* f = fopen(o.trace.c_str(), "wb");
    if (!f) { perror(o.trace.c_str()); return 1; }
    int hdr[9] = {0x63747231, 1, (int)cs.size(), nd, o.steps, TREC, PILLARS, PILLBOXES, NW};
    float con[6] = {DT, ARENA, HP, PB_HEALTH, GUN_RANGE, PB_RANGE};
    fwrite(hdr, sizeof(int), 9, f);
    fwrite(con, sizeof(float), 6, f);
    std::vector<float> rec(recn), fit(nd), ar(3 * PILLARS + 2 * PILLBOXES);
    std::vector<int> out(nd), last(nd), opp(nd);
    for (size_t c = 0; c < cs.size(); c++) {
        CK(cudaMemset(d_rec, 0, recn * sizeof(float)));
        trace_play<<<1, nd>>>(d_ch + c * NW, d_op, (int)os.size(), o.steps, o.trace_seed, o.draw, d_rec, d_out, d_last, d_fit, d_opp, d_ar);
        CK(cudaGetLastError());
        CK(cudaDeviceSynchronize());
        CK(cudaMemcpy(rec.data(), d_rec, recn * sizeof(float), cudaMemcpyDeviceToHost));
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
        int w = 0, l = 0;
        for (int d = 0; d < nd; d++) { w += out[d] == 1; l += out[d] == -1; }
        printf("# trace %s: %d duels, %d won, %d lost, %d drawn\n", cs[c].c_str(), nd, w, l, nd - w - l);
    }
    fclose(f);
    return 0;
#endif
}

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
#endif
    if (!o.trace.empty()) return run_trace(o);

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
        play<<<reps, o.eps, shm>>>(net, hall, hall_n, o.cap, hall_next, ladder, ladder_n, 0.0f, rung, 0.0f, reps, mode, 0.0f, o.steps, s, g, yf, tot);
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
        play<<<o.pop, o.eps, shm>>>(pop, hall, hall_n, o.cap, hall_next, ladder, ladder_n, lad, 0, o.draw, 1, 0, o.spar, o.steps, o.seed, g, fit, tot);
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
        for (int k = 0; k < ladder_n; k++) {
            match(pop, 2, k, o.exam_eps, o.seed ^ 0xe8a3u, gen0 + o.gens, wdl);
            fprintf(f, "%s\t%.4f\t%.4f\t%.4f\t%d\n", names[k].c_str(), wdl[0], wdl[1], wdl[2], n);
            printf("# exam: %s %.3f/%.3f/%.3f\n", names[k].c_str(), wdl[0], wdl[1], wdl[2]);
        }
        fclose(f);
        printf("# exam written to %s (%d milestone(s))\n", o.exam.c_str(), ladder_n);
    }
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
