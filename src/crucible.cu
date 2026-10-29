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

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cmath>
#include <string>
#include <vector>
#include <chrono>
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
#define NIN 32
#define NOUT 3
#define NW (NIN * H + H + H * H + H + H * NOUT + NOUT)
#define RAYS 12
#define PILLARS 6
#define ARENA 64.0f
#define RANGE 40.0f
#define DT 0.1f
#define VMAX 6.0f
#define TURN 2.5f
#define SSPEED 30.0f
#define SLIFE 1.2f
#define RELOAD 1.0f
#define HP 3.0f
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
};

struct Arena {
    float px[PILLARS], py[PILLARS], pr[PILLARS];
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

__device__ inline float wrap(float a) {
    while (a > 3.14159265f) a -= 6.2831853f;
    while (a < -3.14159265f) a += 6.2831853f;
    return a;
}

// The sensors: what tank `me` can know, as 32 numbers in 0..1 or -1..1.
__device__ void sense(const Tank& me, const Tank& en, const Arena& w, float* in) {
#pragma unroll
    for (int r = 0; r < RAYS; r++) {
        float ang = me.a + r * (6.2831853f / RAYS);
        float dx = __cosf(ang), dy = __sinf(ang);
        float tw = ray_wall(me.x, me.y, dx, dy);
#pragma unroll
        for (int p = 0; p < PILLARS; p++) tw = fminf(tw, ray_circle(me.x, me.y, dx, dy, w.px[p], w.py[p], w.pr[p]));
        float te = en.hp > 0.0f ? ray_circle(me.x, me.y, dx, dy, en.x, en.y, TANK_R) : 1e9f;
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
}

// The brain: 32 -> H -> H -> 3, tanh throughout. `w` is in shared memory and
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
#define YARDSTICK Skill{0.12f, 11.0f}

__device__ void scripted(const Tank& me, const Tank& en, Skill k, float* out) {
    float dx = en.x - me.x, dy = en.y - me.y;
    float d = sqrtf(dx * dx + dy * dy);
    float b = wrap(atan2f(dy, dx) - me.a);
    out[0] = fmaxf(-1.0f, fminf(1.0f, 2.0f * b));
    out[1] = d > k.range + 3.0f ? 1.0f : (d < k.range - 3.0f ? -0.5f : 0.0f);
    out[2] = (fabsf(b) < k.tol && d < 35.0f) ? 1.0f : -1.0f;
}

__device__ void drive(Tank& t, const float* act, const Arena& w) {
    if (t.hp <= 0.0f) return;
    t.a = wrap(t.a + act[0] * TURN * DT);
    t.v += (act[1] * VMAX - t.v) * 0.3f;
    t.x = fminf(fmaxf(t.x + __cosf(t.a) * t.v * DT, TANK_R), ARENA - TANK_R);
    t.y = fminf(fmaxf(t.y + __sinf(t.a) * t.v * DT, TANK_R), ARENA - TANK_R);
#pragma unroll
    for (int p = 0; p < PILLARS; p++) {
        float dx = t.x - w.px[p], dy = t.y - w.py[p];
        float d = sqrtf(dx * dx + dy * dy), m = w.pr[p] + TANK_R;
        if (d < m && d > 1e-4f) { t.x = w.px[p] + dx / d * m; t.y = w.py[p] + dy / d * m; t.v *= 0.5f; }
    }
    t.reload -= DT;
    if (act[2] > 0.0f && t.reload <= 0.0f && t.slife <= 0.0f) {
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
}

// ---- one generation's play ----------------------------------------------------------

struct Totals {
    unsigned long long steps; // tank-steps actually played (both tanks)
    unsigned int wins, losses, draws;
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
    int ind = blockIdx.x / reps;
    Rng br{mix(mix(seed, gen), blockIdx.x)};
    Skill skill = YARDSTICK;
    const float* opp = nullptr;
    if (opp_mode == 0) {
        float u = br.u();
        if (u < spar) skill = Skill{br.in(0.05f, 0.6f), br.in(6.0f, 20.0f)};
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
    __syncthreads();

    Rng r{mix(mix(mix(seed, gen), blockIdx.x), threadIdx.x + 1)};
    Arena w;
#pragma unroll
    for (int p = 0; p < PILLARS; p++) { w.px[p] = r.in(14.0f, 50.0f); w.py[p] = r.in(6.0f, 58.0f); w.pr[p] = r.in(1.5f, 3.5f); }
    Tank A, B;
    place(A, r, 3.0f, 14.0f, w);
    place(B, r, 50.0f, 61.0f, w);

    float in[NIN], actA[NOUT], actB[NOUT];
    float aim = 0;
    int t = 0;
    for (; t < steps; t++) {
        if (A.hp <= 0.0f || B.hp <= 0.0f) break;
        sense(A, B, w, in);
        think(wA, in, actA);
        aim += in[29] > 0.0f ? in[29] * (in[28] * in[28] < 0.01f ? 1.0f : 0.2f) : 0.0f;
        if (net_opp) { sense(B, A, w, in); think(wB, in, actB); }
        else scripted(B, A, skill, actB);
        drive(A, actA, w);
        drive(B, actB, w);
        fly(A, B, w);
        fly(B, A, w);
    }
    float dealt = HP - fmaxf(B.hp, 0.0f), taken = HP - fmaxf(A.hp, 0.0f);
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
    __syncthreads();
    if (threadIdx.x == 0) {
        atomicAdd(&fitness[ind], score / (blockDim.x * reps));
        atomicAdd(&tot->wins, sw); atomicAdd(&tot->losses, sl); atomicAdd(&tot->draws, sd);
    }
}

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
    pop[i] = 0.3f * gauss(mix(seed, (uint32_t)i));
}

// ---- the host: setup, the loop, the numbers ---------------------------------------------

struct Opt {
    int pop = 8192, eps = 256, steps = 600, gens = 100, elite = 16, every = 5, cap = 4096, yard = 10, yard_eps = 4096;
    float sigma = 0.03f, spar = 0.5f, lad = 0.2f, draw = -1.0f;
    int exam_eps = 2048;
    uint32_t seed = 1;
    std::string out = "", save = "", load = "", exp = "", imp = "", ladder = "", exam = "";
    bool bench = false;
};

static void usage() {
    fprintf(stderr,
            "crucible [--pop N] [--eps N] [--steps N] [--gens N] [--elite N] [--every K] [--cap N]\n"
            "         [--sigma S] [--spar P] [--seed N] [--yard K] [--out FILE.tsv] [--save FILE] [--load FILE] [--bench]\n"
            "         [--export CHAMP.bin] [--import A.bin,B.bin,...] [--ladder M1.bin,M2.bin,...] [--lad P]\n"
            "         [--exam FILE.tsv] [--exam-eps N] [--draw V]\n"
            "  --export writes the last generation's champion (its weights, as floats);\n"
            "  --import puts other islands' champions into the hall of fame first;\n"
            "  --ladder loads milestones (oldest first): opponents with chance --lad, and a yardstick;\n"
            "  --exam writes the final champion's score against each milestone and the yardstick;\n"
            "  --draw is a drawn duel's worth in training (a win is +2, a loss -2, besides damage).\n");
    exit(2);
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
    printf("# device %s, sm_%d%d, %d SMs, %.0f MHz, %.1f GB (%.1f free)\n", dp.name, dp.major, dp.minor, dp.multiProcessorCount,
           dp.clockRate / 1000.0, total_b / 1e9, free_b / 1e9);
    printf("# net 32-%d-%d-3: %d weights (%.1f KB); play: %d registers/thread, %zu B shared/block, %d blocks of %d per SM (occupancy %.0f%%)\n",
           H, H, NW, NW * 4 / 1024.0, fa.numRegs, shm, per_sm, o.eps, 100.0 * per_sm * o.eps / dp.maxThreadsPerMultiProcessor);

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
        int hdr[7];
        if (fread(hdr, sizeof(int), 7, f) != 7 || hdr[0] != 0x63727532 || hdr[1] != NW || hdr[2] != o.pop || hdr[3] != o.cap) {
            fprintf(stderr, "%s: not a crucible state for this net, --pop and --cap\n", o.load.c_str());
            return 1;
        }
        gen0 = hdr[4]; hall_n = hdr[5]; hall_next = hdr[6];
        std::vector<float> h((size_t)o.pop * NW);
        if (fread(h.data(), 4, h.size(), f) != h.size()) return 1;
        CK(cudaMemcpy(pop, h.data(), pop_b, cudaMemcpyHostToDevice));
        h.resize((size_t)o.cap * NW);
        if (fread(h.data(), 4, h.size(), f) != h.size()) return 1;
        CK(cudaMemcpy(hall, h.data(), hall_b, cudaMemcpyHostToDevice));
        fclose(f);
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
            std::vector<float> w(NW);
            FILE* f = fopen(path.c_str(), "rb");
            if (!f || fread(w.data(), 4, NW, f) != (size_t)NW) {
                fprintf(stderr, "# import %s: not a champion of this net, skipped\n", path.c_str());
                if (f) fclose(f);
                continue;
            }
            fclose(f);
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
        std::vector<float> w(NW);
        FILE* f = fopen(path.c_str(), "rb");
        if (!f || fread(w.data(), 4, NW, f) != (size_t)NW) {
            fprintf(stderr, "# ladder %s: not a champion of this net, skipped\n", path.c_str());
            if (f) fclose(f);
            continue;
        }
        fclose(f);
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
    if (out && ftell(out) == 0) fprintf(out, "gen\tbest\tmean\tplay_s\tsteps_per_s\tnet_gflops\thall\tyard_win\tyard_draw\tyard_loss\tladder_score\tladder_top\n");
    printf("gen\tbest\tmean\tplay_s\tMsteps/s\tnet GFLOP/s\thall\tyardstick w/d/l\tladder all/top\n");

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

        printf("%d\t%.3f\t%.3f\t%.3f\t%.1f\t%.0f\t%d\t%s\t%s\n", g, best, mean, ms / 1000.0, sps / 1e6, gfl, hall_n, yard, ladc);
        fflush(stdout);
        if (out) {
            fprintf(out, "%d\t%.4f\t%.4f\t%.4f\t%.0f\t%.1f\t%d\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\n", g, best, mean, ms / 1000.0, sps, gfl, hall_n, yw, yd, yl, ls, lt);
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
        int hdr[7] = {0x63727532, NW, o.pop, o.cap, gen0 + o.gens, hall_n, hall_next};
        fwrite(hdr, sizeof(int), 7, f);
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
